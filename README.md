# sqlay

SQL Inlay.

`sqlay` is a Rust CLI for writing plain SQL files while generating typed target
language builders. The current supported workflow focuses on MySQL 8.x `SELECT`
queries, MySQL mutation builders, inline `Param` value binding, Slot/Fragment
composition, Repeat list expansion, and TypeScript SQL builder generation.

## Why sqlay?

`sqlay` is `SQL` + `inlay`. SQL remains the readable base material, while explicit
composition openings such as Slots accept validated Fragments as inlays instead of
arbitrary string concatenation.

See [`docs/current-scope.md`](./docs/current-scope.md),
[`docs/vision.md`](./docs/vision.md), and
[`docs/architecture.md`](./docs/architecture.md) for the active product direction
and architecture. The completed initial MVP baseline remains in
[`docs/mvp.md`](./docs/mvp.md).

## Usage

Make the `sqlay` command available before starting a project. From a local
checkout, install the binary into Cargo's bin directory with:

```sh
cargo install --locked --path .
```

Cargo usually installs binaries into `~/.cargo/bin`; make sure that directory is
on `PATH`. During local development, you can also run the same CLI through Cargo:

```sh
cargo run --locked -- init
cargo run --locked -- check
```

Create the starter project configuration from the directory that should contain
`sqlay.config.json`:

```sh
sqlay init
```

`sqlay init` writes a starter `sqlay.config.json` and refuses to overwrite an
existing config file. The starter config uses the supported project shape:

```json
{
  "source": {
    "include": ["sql/**/*.sql"],
    "exclude": []
  },
  "output": {
    "dir": "src/generated/sqlay"
  },
  "database": {
    "dialect": "mysql",
    "urlEnv": "DATABASE_URL"
  },
  "target": {
    "language": "typescript"
  }
}
```

The config is parsed as JSON with comments and trailing commas allowed. Source and
output paths are resolved relative to the directory containing
`sqlay.config.json`. Matched SQL files must remain inside that configuration
directory so generated paths can be preserved relative to it. Place
`sqlay.config.json` at the project root when source SQL lives in sibling
directories such as `sql/` next to `configs/`. For `check` and `compile`, when
`--config` is omitted, sqlay starts from the current working directory and searches
upward for `sqlay.config.json`.

Run a database-backed dry run with:

```sh
DATABASE_URL='mysql://user:password@host:3306/database' sqlay check
```

`sqlay check` loads the config, reads SQL files, validates MySQL 8.x `SELECT`
queries, resolves inline `Param` inputs, looks up MySQL metadata, and builds
generated TypeScript in memory without writing files. The database URL is read from
the process environment variable named by `database.urlEnv`; the CLI does not
implicitly load `.env` files. With the starter config, either export
`DATABASE_URL` before running sqlay commands or prefix a single command with
`DATABASE_URL='mysql://user:password@host:3306/database'`.

Write generated TypeScript with:

```sh
DATABASE_URL='mysql://user:password@host:3306/database' sqlay compile
```

`sqlay compile` runs the same pipeline as `check` and writes TypeScript SQL
builder files under `output.dir`. Generated paths preserve each input SQL path
relative to the configuration directory; for example, `sql/books.sql` generates
`src/generated/sqlay/sql/books.ts` with the starter config. Normal `compile`
overwrites same-path generated files. Use `sqlay compile --clean` to also remove
stale managed generated files.

Generated TypeScript includes a generated-code header, input types, database-backed
row and output types for SELECT queries, and builder functions that return SQL text
plus readonly parameter arrays. Mutation builders generate input types and builder
functions, but they do not generate row types, output types, execution functions, or
driver-specific result wrappers. Slotless and direct-Param-only builders use
readonly tuples for fixed parameter shapes; dynamic builders use readonly arrays
when selected branches can change the parameter shape at runtime. Generated code
does not execute SQL or depend on a database driver.

Dynamic values are written with paired inline `Param` markers around sample SQL
expressions:

```sql
/* @sqlay
{
  type: query
  id: findBook
}
*/
SELECT b.title
FROM books AS b
WHERE b.isbn = /* @sqlay { type: param id: isbn } */
  '9780131103627'
  /* @sqlay { type: paramEnd } */;
```

Raw `?` placeholders are not accepted in source SQL; use `Param` markers so the SQL
file remains readable in database tools and the generated builder receives typed
input. Param type inference uses qualified column context such as `b.isbn`; add a
`valueType` override when a Param is not next to a supported qualified column.

For nullable inputs, keep `valueType` to a sqlay CoreType name and add
`nullable: true` instead of writing a TypeScript union:

```sql
WHERE b.published_at < /* @sqlay { type: param id: publishedBefore valueType: datetime nullable: true } */
  '2026-01-01 00:00:00'
  /* @sqlay { type: paramEnd } */
```

```ts
export type findBook_Input = {
  publishedBefore: string | null;
};
```

Optional direct Param input properties are not currently supported because they
imply SQL structure changes. Use a nullable sentinel pattern, separate queries for
distinct SQL shapes, or Slot/Fragment composition for supported dynamic SQL.

## Mutation Builders

Use `type: mutation` for MySQL `INSERT`, `UPDATE`, `DELETE`, and `REPLACE` builders.
Mutation builders return SQL text and params only, keeping transaction handling,
execution result interpretation, and database driver choice in application code:

```sql
/* @sqlay
{
  type: mutation
  id: createOrder
}
*/
INSERT INTO bookstore_orders (
  customer_id,
  order_number,
  status,
  currency,
  placed_at
) VALUES (
  /* @sqlay { type: param id: customerId } */
  1000
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: orderNumber } */
  'BK-2000'
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: status } */
  'draft'
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: currency } */
  'USD'
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: placedAt } */
  '2026-04-20 12:00:00.000000'
  /* @sqlay { type: paramEnd } */
);
```

`check` and `compile` validate mutation SQL and infer Param types from schema
metadata, but they never execute mutation statements. Use an explicit SELECT builder
when application code needs the row created or updated by a mutation. See
[`docs/mutation-execution.md`](./docs/mutation-execution.md) for `mysql2/promise`
execution examples covering `insertId`, affected row counts, transactions,
multi-row inserts, upserts, and `REPLACE`.

## Slot/Fragment Composition

Fragments are global source units that hold reusable SQL insertion text. Slots are
query-local insertion points that choose from the global fragments named in their
`targets` list. Initial slots are optional single-select slots: a caller may select
one target fragment, or omit the slot input to insert an empty string.

Fragment-only SQL files are valid inputs, but they do not produce path-matching
`.ts` files. Cross-file fragments are embedded into the generated TypeScript file
for each query that uses them.

```sql
/* @sqlay
{
  type: fragment
  id: staffPicksOnly
}
*/
  AND EXISTS (
    SELECT 1
    FROM bookstore_book_categories AS filter_bc
    INNER JOIN bookstore_categories AS filter_c
      ON filter_c.id = filter_bc.category_id
    WHERE filter_bc.book_id = b.id
      AND filter_c.slug = 'staff-picks'
  )

/* @sqlay
{
  type: fragment
  id: byBookFormat
}
*/
  AND b.format = /* @sqlay { type: param id: format } */
    'paperback'
    /* @sqlay { type: paramEnd } */

/* @sqlay
{
  type: query
  id: listAvailableBooks
}
*/
SELECT b.id, b.title
FROM bookstore_books AS b
WHERE b.stock_quantity > 0
/* @sqlay { type: slot id: discoveryFilter targets: [staffPicksOnly, byBookFormat] } */;
```

Generated slot inputs use `$fragment` as the branch discriminant. Fragment Params
are nested under the selected slot branch instead of being lifted to the query
input top level:

```ts
export type listAvailableBooks_Input = {
  discoveryFilter?: { $fragment: "staffPicksOnly" } | {
    $fragment: "byBookFormat";
    format: string;
  };
};

listAvailableBooks({
  discoveryFilter: { $fragment: "byBookFormat", format: "paperback" },
});

listAvailableBooks();
```

During `check` and `compile`, sqlay validates dynamic SQL up to the 256 validation
case limit. Slot expansion variants and Repeat representative cases both
contribute to that count. All Slot variants must keep the same result row shape and
effective cardinality as the all-slots-unselected base variant. Fragment-local
slots and required slots are reserved for future work.

## Repeat Lists

Use paired inline `repeat` and `repeatEnd` markers when one SQL list item should be
expanded from a caller-provided array. The Repeat range wraps one item template;
ordinary SQL owns the surrounding list syntax such as `IN (` and `)`.

```sql
AND b.id IN (
  /* @sqlay { type: repeat id: ids separator: ", " } */
  /* @sqlay { type: param id: id valueType: int64 } */
  100
  /* @sqlay { type: paramEnd } */
  /* @sqlay { type: repeatEnd } */
)
```

Generated Repeat inputs are non-empty readonly arrays of item objects. Even a
single-Param Repeat item uses an object item instead of a scalar array:

```ts
export type listBooks_Input = {
  ids: readonly [{ id: string }, ...{ id: string }[]];
};

listBooks({ ids: [{ id: "100" }, { id: "102" }] });
```

Generated builders also reject empty arrays at runtime, so JavaScript callers or
`any` casts cannot produce invalid SQL such as `IN ()`. Repeat does not define
`maxItems` in the initial design; database and driver limits for very large input
arrays remain caller responsibility.

For bulk `VALUES`, put the Repeat range around one row tuple and provide the comma
separator explicitly as raw SQL text:

```sql
INSERT INTO bookstore_order_items (
  order_id,
  book_id,
  quantity
) VALUES
/* @sqlay { type: repeat id: items separator: "," } */
(
  /* @sqlay { type: param id: orderId } */
  5000
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: bookId } */
  100
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: quantity } */
  1
  /* @sqlay { type: paramEnd } */
)
/* @sqlay { type: repeatEnd } */;
```

`separator` is required and inserted exactly as raw SQL text between expanded
items; sqlay does not infer commas, spaces, or newlines. Repeat can appear in
SELECT queries, mutation builders, and fragments selected through Slots. It
changes SQL builder input and parameter emission only; it is not a generated
execution or batch helper.

## Local MySQL

The repository includes a Docker Compose service for local MySQL 8.x development:

```sh
script/mysql-up.sh
```

The service uses deterministic development-only credentials and waits for the
container healthcheck before the command exits.

Use this connection URL for local checks:

```sh
export DATABASE_URL='mysql://sqlay:sqlay@127.0.0.1:3306/sqlay'
```

The local MySQL service starts an empty development database. Example and fixture
checks load their own prefix-scoped schema and seed data each time they run. Reset
the database volume with:

```sh
script/mysql-reset.sh
```

Stop the service without removing the database volume with:

```sh
script/mysql-down.sh
```

## Local Checks

Run the same non-database baseline checks used by CI with:

```sh
npm ci
script/check-baseline.sh
```

Type-check committed generated TypeScript artifacts directly with:

```sh
npm run typecheck:examples
npm run typecheck:fixtures
```

Run the MySQL-backed example E2E check against a running MySQL service with:

```sh
DATABASE_URL='mysql://sqlay:sqlay@127.0.0.1:3306/sqlay' script/check-examples.sh
```

Run the MySQL-backed fixture checks with:

```sh
DATABASE_URL='mysql://sqlay:sqlay@127.0.0.1:3306/sqlay' script/check-mysql-fixtures.sh
```
