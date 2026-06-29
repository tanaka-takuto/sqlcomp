# Architecture

`sqlay` is implemented as a Rust CLI with a small set of components connected by
explicit intermediate representations. The components are named by responsibility so
their database and target-language dependencies stay visible.

The authoritative product rules live in [Vision](./vision.md), and the active
supported scope lives in [Current Scope](./current-scope.md). The completed initial
MVP baseline is recorded in [Initial MVP Baseline](./mvp.md).

## Crate Layout

`sqlay` is a Cargo workspace. Component boundaries are represented as separate
workspace crates, not only as Rust modules, so dependency direction is enforced by
`Cargo.toml` path dependencies.

The root `src/` directory belongs only to the final `sqlay` binary package.
Reusable implementation crates live under `crates/`, which is the conventional
workspace layout for multiple Rust packages in one repository.

Runtime flow and dependency direction are intentionally different. Runtime flow
moves from intake toward generation, but crate dependencies point inward. Inner
crates never depend on outer crates.

```text
sqlay binary crate
  -> sqlay-cli

sqlay-cli
  -> sqlay-app
  -> sqlay-adapters

sqlay-adapters
  -> sqlay-app
  -> sqlay-core

sqlay-app
  -> sqlay-core

sqlay-core
  -> no sqlay-* dependencies
```

Only `sqlay-cli` may depend on both `sqlay-app` and `sqlay-adapters`.
`sqlay-cli` is the composition root: it wires concrete adapters into application
ports. `sqlay-adapters` groups infrastructure adapters as modules such as
`config_jsonc`, `source_fs`, `dialect_mysql`, `metadata/<database>/<driver>`,
`target`, and `output_fs`. The current sqlx-backed MySQL metadata adapter lives
under `metadata/mysql/sqlx`. Target-language adapters live under
`target/<language>` directories, such as `target/typescript`, with shared target
helpers owned by `target`.

Adapter modules implement ports from `sqlay-app` and exchange only `sqlay-core`
types. `sqlay-app` owns use cases and port traits. `sqlay-core` owns shared domain
vocabulary and language-neutral IR. A new dependency edge between workspace crates
is an architecture decision, not an incidental import.

## Component Flow

```text
CLI Driver
  -> Config Loader
  -> Compilation Plan

Source Intake
  -> RawSourceUnit(Query | Mutation | Fragment)
  -> inline Param, Slot, and Repeat usage metadata

Dialect Analyzer
  RawQuery + dialect rules
  -> AnalyzedQuery

Mutation Analyzer
  RawMutation + dialect rules
  -> AnalyzedMutation

Metadata Provider
  RawQuery + database connection
  -> DbQueryMetadata

Schema Metadata Provider
  RawMutation + information_schema
  -> DbMutationMetadata

Application Use Case + Core IR
  RawSourceUnit + analysis + metadata
  -> CompiledBuilder(Query | Mutation)

Target Generator
  CompiledBuilder values
  -> generated files
```

This structure avoids a direct `database dialect x target language` implementation
matrix. Database-specific logic maps database behavior into Core IR. Target
generators map Core IR into language-specific code.

Mutation analysis is intentionally separate from SELECT query analysis. SELECT
queries can use database describe metadata for result columns. Mutations must never
be executed during `check` or `compile`, so their metadata path is limited to schema
metadata needed for Param type inference.

## Diagnostics and Errors

Components that can fail with user-facing errors return shared diagnostic
primitives from `sqlay-core` instead of formatting final CLI output themselves.
Diagnostics carry a human-readable message and may include file path and one-based
source location context when that information is available.

The CLI remains responsible for converting diagnostic reports into stderr output
and process exit codes. Application services and adapters should return structured
diagnostics to the CLI boundary.

## CLI Driver

The CLI Driver owns command selection, configuration discovery, process environment
access, and user-facing diagnostics. It should not parse SQL or generate TypeScript
directly.

The CLI crate is also the composition root. It may depend on `sqlay-app`,
`sqlay-core`, and all concrete adapter crates. No inner crate may depend on the CLI
crate.

The supported command surface is:

- `init` writes a starter `sqlay.config.json`.
- `check` runs the full compile pipeline without writing generated files.
- `compile` writes generated TypeScript files.

`check` and `compile` use the same analysis and generation pipeline so CI and local
generation validate the same behavior.

## Config Loader

Config Loader resolves the project configuration before Source Intake runs.

Responsibilities:

- find `sqlay.config.json` from the current working directory upward when
  `--config` is not provided.
- parse JSON with comments and trailing commas allowed.
- validate the supported values for source, output, database, and target settings.
- validate TypeScript target type mapping override settings when
  `target.language` is `typescript`.
- resolve source and output paths relative to the configuration file directory.
- require matched source files to remain inside the configuration directory, so
  output paths can be derived relative to one stable project root.
- read the database URL from the process environment using `database.urlEnv`.

The CLI does not implicitly load `.env` files.

## Compilation Plan

The Compilation Plan is the resolved work order produced from configuration. It is
not a semantic SQL representation.

Responsibilities:

- expand `source.include` and `source.exclude` into the input SQL file set.
- keep each input file path relative to the configuration file directory.
- carry the resolved output directory.
- carry the database URL and target selection for downstream components.

Projects with SQL files in sibling directories should place `sqlay.config.json` at
their common root. A nested config such as `configs/sqlay.qa.json` cannot use
`../sql/**/*.sql` to pull sources from outside the config directory without
breaking the config-relative output path model.

## Source Intake

Source Intake reads SQL files and extracts sqlay source units. It does not decide
whether the SQL is valid MySQL, PostgreSQL, or another dialect.

Responsibilities:

- read `.sql` files.
- find `@sqlay` comments.
- parse Hjson metadata payloads.
- split files into source-ordered query, mutation, and fragment units.
- preserve each query, mutation, or fragment body's raw SQL string.
- collect independent source-intake diagnostics across discovered SQL files before
  returning failure.

Source Intake is not fully independent from SQL syntax because it must scan SQL
comments and avoid corrupting string literals or comment-like text. However, it
should avoid database semantic decisions. It should produce `RawSourceUnit` values
for configured analyzers and application validation to interpret.

The canonical query annotation form is:

```sql
/* @sqlay
{
  type: query
  id: listUsers
}
*/
SELECT id, name FROM users;
```

The canonical mutation annotation form is:

```sql
/* @sqlay
{
  type: mutation
  id: createUser
}
*/
INSERT INTO users (email, name)
VALUES (
  /* @sqlay { type: param id: email } */
  'ada@example.test'
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: name } */
  'Ada'
  /* @sqlay { type: paramEnd } */
);
```

For query annotations:

- `type: query` is required.
- `id` is required and is never inferred.
- `id` must match `^[A-Za-z_][A-Za-z0-9_]*$`.
- `id` must be unique across the full compile run.
- `cardinality` is optional and may override SELECT cardinality inference.

For mutation annotations:

- `type: mutation` is required.
- `id` is required and is never inferred.
- `id` must match `^[A-Za-z_][A-Za-z0-9_]*$`.
- `id` must be unique across the full compile run.
- no statement kind, result, or execution metadata is accepted initially.

For `Param` intake, inline `type: param` and `type: paramEnd` annotations are
recognized inside query, mutation, and fragment bodies. For analysis and
generation, each Param range is replaced with one MySQL positional placeholder.
Raw `?` placeholders are rejected in source SQL.

For `Fragment` intake, `type: fragment` starts a global source unit with an
explicit `id` and a body that ends before the next global `query`, `mutation`, or
`fragment` annotation. Fragment source units preserve their raw SQL body exactly.
A fragment is valid only in insertion context; query and mutation validation decide
whether a fragment composes into valid SQL at each slot.

For `Slot` intake, query-local and mutation-local `type: slot` markers are parsed,
validated, recorded as zero-width insertion points, and removed from the SQL text
used for downstream analysis. Initial slots remain optional single-select slots:
each unique Slot ID contributes one unselected choice plus one choice per target
fragment.

For `Repeat` intake, inline `type: repeat` and `type: repeatEnd` annotations are
recognized inside query, mutation, and fragment bodies. A Repeat range encloses one
variable-length list item template and has required `id` and `separator` metadata.
The surrounding list syntax, such as `IN (` and `)`, or the `VALUES` keyword, stays
outside the Repeat range. Repeat ranges may contain Param ranges, but they may not
contain Slot markers or nested Repeat ranges. Param ranges may not contain Repeat
markers. Repeat ranges without any Param markers are rejected.

Repeated Slot IDs in one query or mutation are accepted only when their `targets`
arrays match exactly, including order. Repeated Repeat IDs share one generated
array input when their item Param ID set, CoreType, and nullability are compatible.
The first Repeat occurrence fixes generated item field order; later occurrences may
use different SQL text, separators, and Param occurrence order.

Direct Param IDs, Slot IDs, and Repeat IDs collide when they would share the same
generated top-level input namespace. Fragment Params and Fragment Repeats are
nested inside selected slot branch objects and share that branch object's namespace.
Repeat item Params are nested inside each Repeat item object.

Fragments that are not referenced by any Slot target produce non-fatal warnings.

## Dialect and Mutation Analyzers

The Dialect Analyzer interprets a `RawQuery` as SQL for one configured database
dialect. The Mutation Analyzer interprets a `RawMutation` under the same dialect
boundary.

The currently supported dialect is MySQL 8.x.

Query analyzer responsibilities:

- parse the raw SQL according to dialect rules.
- reject unsupported statement forms.
- verify that each query block contains exactly one `SELECT` statement.
- infer dialect-dependent query facts such as `LIMIT 1` cardinality.
- produce `AnalyzedQuery` without target-language concerns.

Mutation analyzer responsibilities:

- parse the raw SQL according to dialect rules.
- reject unsupported statement forms.
- verify that each mutation block contains exactly one supported mutation
  statement.
- accept initial `INSERT`, `UPDATE`, `DELETE`, and `REPLACE` forms defined by
  [ADR 0010](./adr/0010-define-initial-mysql-mutation-builder-support.md).
- reject multi-table `UPDATE` and `DELETE`, `INSERT ... SELECT`,
  `REPLACE ... SELECT`, top-level CTE mutations, `CALL`, `LOAD DATA`, `TRUNCATE`,
  DDL, transaction control, administrative statements, and multi-statement units.
- require `WHERE` on `UPDATE` and `DELETE` without attempting semantic predicate
  safety analysis.
- expose mutation facts such as statement kind and target table information for
  application validation and Param inference.

Future PostgreSQL or SQLite support should add new dialect analyzers rather than
branching inside target generators.

## Metadata Provider

The Metadata Provider obtains database metadata for analyzed source units.

For SELECT queries, the provider connects to MySQL 8.x and derives result column
metadata without fetching user data. The Rust database client is `sqlx`.

Query metadata responsibilities:

- connect to the configured database.
- describe a SELECT query without fetching user data rows.
- return database-native column names, database types, and nullability metadata.
- read `information_schema.columns` metadata used for direct column-context input
  type inference and schema-backed type mapping.
- preserve enough schema identity to distinguish current-database `table.column`
  references from explicit MySQL `database.table.column` references.
- read native column declarations such as `COLUMN_TYPE` when Core type metadata
  needs details beyond a broad database type name, including MySQL `ENUM` values.

Mutation metadata responsibilities:

- connect to the configured database only for schema metadata.
- read `information_schema.columns` metadata used for supported direct
  column-context input type inference and schema-backed type mapping.
- preserve enough schema identity to distinguish current-database `table.column`
  references from explicit MySQL `database.table.column` references.
- never execute mutation SQL.
- never rely on rollback-based execution to infer mutation behavior.

See also:

- [ADR 0001: Use MySQL 8.x as the MVP dialect](./adr/0001-use-mysql-8-for-mvp.md)
- [ADR 0010: Define Initial MySQL Mutation Builder Support](./adr/0010-define-initial-mysql-mutation-builder-support.md)

## Application Use Cases and Ports

Application use cases coordinate the supported workflow and own the port traits that
adapters implement.

Responsibilities:

- define ports such as config loading, source reading, dialect analysis, metadata
  lookup, target generation, and generated-file writing.
- coordinate `init`, `check`, and `compile` workflows.
- depend only on `sqlay-core`.
- avoid filesystem, database, SQL parser, and TypeScript formatting implementation
  details.

Application flow should preserve source order across mixed query and mutation
builders. Fragment-only files do not generate path-matching TypeScript files, but
their fragments may be embedded into query or mutation builder files that use them.

Dynamic SQL validation is counted in validation cases, not only Slot variants. A
validation case is the product of Slot selection variants and Repeat representative
cases. Initial Repeat validation uses one representative case, a two-item expansion
that exercises the separator. The validation case limit remains 256.

## Compilation Core

Compilation Core is the innermost crate. It owns shared domain vocabulary and
language-neutral Core IR. It must not depend on source intake, dialect analyzers,
metadata providers, target generators, or the CLI.

IR means intermediate representation: an internal data structure that is no longer
raw SQL input, but is not yet TypeScript, Go, Rust, or any other generated language.

SELECT query IR and mutation IR should stay separate because they have different
contracts. Queries have result rows and cardinality. Mutations do not.

Example Core IR shape:

```rust
enum CompiledBuilder {
    Query(CompiledQuery),
    Mutation(CompiledMutation),
}

struct CompiledQuery {
    id: QueryId,
    sql: String,
    cardinality: Cardinality,
    input: Vec<InputField>,
    params: Vec<ParamBinding>,
    row: Vec<ResultColumn>,
}

struct CompiledMutation {
    id: MutationId,
    sql: String,
    kind: MutationKind,
    input: Vec<InputField>,
    params: Vec<ParamBinding>,
}
```

Dynamic Core IR should represent Slot and Repeat emission without merging SELECT
query IR and mutation IR. Repeat definitions need enough language-neutral
information for target generators to render non-empty array inputs, runtime
empty-array guards, separators, item SQL segments, and Param bindings in emitted
SQL order.

Database-specific type mapping should stop at Core IR:

```text
MySQL BIGINT -> CoreType::Int64
PostgreSQL int8 -> CoreType::Int64
MySQL ENUM('draft', 'paid') -> Enum value type ['draft', 'paid']
```

Target-language type mapping should start from Core IR:

```text
CoreType::Int64 -> TypeScript string
CoreType::Int64 -> Go int64
Enum value type ['draft', 'paid'] -> TypeScript "draft" | "paid"
```

This keeps MySQL-to-TypeScript, PostgreSQL-to-TypeScript, MySQL-to-Go, and
PostgreSQL-to-Go from becoming separate hard-coded paths.

Core metadata should be conservative:

- database nullability metadata is used when available.
- unknown nullability maps to nullable output for SELECT result rows.
- precision-sensitive types such as `BIGINT`, `DECIMAL`, and date/time values
  should avoid lossy JavaScript conversions in the TypeScript target generator.
- schema-backed MySQL `ENUM` values should be represented in language-neutral Core
  metadata, not as TypeScript-only generator state.

## Target Generator

Target Generators convert Core IR into generated files for a target language. They
should not parse or reinterpret database-specific SQL syntax. The SQL text inside a
generated file may be MySQL or another dialect, but the generator treats that SQL
as validated text carried by the Core IR.

The supported target generator emits TypeScript SQL builder code. Generated code
returns SQL text and parameter arrays, not database execution behavior.

TypeScript type mapping overrides change generated type annotations only. They do
not add runtime result parsing, input validation, driver configuration, or SQL
rewrites. The generator resolves the configured TypeScript type surface from Core
metadata, schema-backed enum defaults, and ordered override rules defined by
[ADR 0012](./adr/0012-define-configurable-typescript-type-mapping-overrides.md).

Generated TypeScript is emitted per SQL file while preserving the input path
relative to the directory containing `sqlay.config.json`. If one SQL file contains
multiple queries and mutations, the corresponding TypeScript file contains multiple
generated builder functions and type aliases in source order.

Generated names are not case-converted. The source-unit `id` is used exactly as
written, with fixed suffixes for generated TypeScript types.

For a SELECT query:

```ts
export type listUsers_Input = Record<string, never>;

export type listUsers_Row = {
  id: number;
  name: string | null;
};

export type listUsers_Output = listUsers_Row[];

export function listUsers(
  _input: listUsers_Input = {},
): { sql: string; params: readonly [] } {
  return {
    sql: "SELECT id, name FROM users;",
    params: [] as const,
  };
}
```

For a mutation:

```ts
export type createUser_Input = {
  email: string;
  name: string;
};

export function createUser(
  input: createUser_Input,
): { sql: string; params: readonly [string, string] } {
  return {
    sql: "INSERT INTO users (email, name) VALUES (?, ?);",
    params: [input.email, input.name] as const,
  };
}
```

Mutation builders do not generate `Row` or `Output` aliases because sqlay does not
execute statements or own driver result objects.

For Slot builders, generated input types add each unique Slot ID as an optional
top-level property. Each Slot property is a `$fragment` discriminated object or
union of objects in `targets` order. Fragment Params are nested inside the selected
branch object and are not exported as independent fragment input aliases. Generated
Slot builder functions use a private `SqlParam` alias, append SQL segments with
`sqlParts.push(...)`, branch on `input.slotId?.$fragment` without a default case,
and return `sqlParts.join("")` with Params appended in expanded SQL order.

For Repeat builders, generated input types add each unique Repeat ID as a required
non-empty readonly array of inline item objects. Repeat item type aliases are not
exported. A single-Param Repeat item still uses an object item, such as
`ids: readonly [{ id: string }, ...{ id: string }[]]`, not a scalar array. Builders
with any Repeat return `params: readonly SqlParam[]`, because runtime input length
changes the number of placeholders. Generated builders check each emitted Repeat
input for an empty array before expanding it.

Configured type annotation overrides apply to result row fields, direct Param input
fields, direct Repeat item fields, and fixed params tuple element types. Direct
Repeat item field overrides are scoped by builder ID, Repeat ID, and item field
name so multiple Repeat inputs may reuse the same item field names. Overrides
preserve nullability and do not make dynamic Slot or Repeat params arrays more
precise: builders with dynamic params continue to return `readonly SqlParam[]` with
a private `type SqlParam = unknown`.

Custom project types may be imported with type-only imports from non-relative
module specifiers. The generator de-duplicates identical imports per generated
file, rejects same-name imports from different modules, and does not create
automatic import aliases.

Repeat SQL generation uses the same `sqlParts` and `params` append model as Slot
generation. A loop emits each item, pushes the Repeat separator before every item
after the first, and appends item Params in that occurrence's SQL placeholder
order. Repeat inputs inside optional Slot branches are checked and expanded only
when that branch is selected.

Generated SQL must be emitted as a valid JavaScript string literal. The TypeScript
target generator should escape the SQL text rather than embedding raw SQL in an
unescaped template literal, because MySQL backtick identifiers and SQL text
containing `${...}` must not break generated TypeScript. Examples use ordinary
double-quoted string literals; multiline SQL may use any representation that is
semantically equivalent after JavaScript string escaping.

Generated files include a generated-code header. `compile` treats the configured
output directory as generated output and overwrites same-path files. Stale generated
files are removed only when `compile --clean` is used.

## Development and Integration Checks

The project should keep local and CI checks aligned. Rust formatting, linting, and
unit tests remain the external-service-free baseline checks. MySQL-backed checks are
separate because they require a running MySQL 8.x database and prefix-scoped schema
reset.

Examples and fixtures have different responsibilities. `examples/` contains
user-facing sample projects with generated TypeScript output that is actual compiler
output. `fixtures/` contains implementation-focused test inputs and expected
artifacts for coverage, edge cases, and diagnostics.

DB-backed generated-output checks should regenerate examples and fixtures in
temporary directories, compare the generated output byte for byte with committed
expected artifacts, and type-check the generated TypeScript. These checks should not
use Git working-tree diffs as their oracle.

Rust tests should follow the conventional crate layout:

- unit tests live inside the module they test, usually in a `#[cfg(test)] mod tests`
  block near the implementation inside the owning crate.
- integration tests live outside the crate source tree under that package's
  `tests/` directory and exercise public crate APIs the way an external caller
  would.

This placement is intentional. Component-local behavior should be tested from
inside the component module so private helpers can stay private. Cross-component,
CLI, generated-output, filesystem, and database-backed behavior should be tested
from the appropriate package-level `tests/` directory so the test boundary matches
the public library or binary behavior.

Tests should protect product behavior or a stable repository contract rather than
incidental test harness internals. Product-facing tests include SQL parsing,
analysis, generation, CLI diagnostics and summaries, generated output shape,
example and fixture type-checking, and documented check-script exit behavior. Tests
may use fakes, mocks, and temporary projects to make those contracts deterministic,
but assertions should stay focused on the contract under test.

Generated TypeScript should be type-checked in CI with `tsc --noEmit`. This
verifies that generated builders are usable in ordinary TypeScript projects without
adding runtime dependencies.

See also:

- [ADR 0006: Define MVP CLI, Config, and Generation Workflow](./adr/0006-define-mvp-cli-config-and-generation-workflow.md)
- [ADR 0002: Use TypeScript SQL builders as the first target generator](./adr/0002-use-typescript-target-generator-first.md)
- [ADR 0005: Do not automatically transform generated names](./adr/0005-do-not-transform-generated-names.md)
- [ADR 0007: Use examples and fixtures as generated E2E artifacts](./adr/0007-use-examples-and-fixtures-as-generated-e2e-artifacts.md)
- [ADR 0010: Define Initial MySQL Mutation Builder Support](./adr/0010-define-initial-mysql-mutation-builder-support.md)
