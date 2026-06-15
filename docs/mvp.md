# MVP

The first `sqlcomp` implementation is intentionally small. It should prove the full
compile path from SQL files to typed TypeScript SQL builders without implementing
dynamic query composition.

## Scope

The MVP supports:

- MySQL 8.x.
- TypeScript generation.
- `sqlcomp.config.json` project configuration.
- `init`, `check`, and `compile` CLI commands.
- query annotations with Hjson `@sqlcomp` comments.
- `SELECT` statements only.
- one or more queries per SQL file.
- exactly one SQL statement per query block.
- output TypeScript files generated per SQL file while preserving
  config-file-relative directory structure.

The MVP does not support:

- `INSERT`, `UPDATE`, `DELETE`, DDL, `CALL`, or multi-statement query blocks.
- `Param`, `Slot`, or `Fragment`.
- generated database execution functions.
- automatic naming transformation.
- non-MySQL dialects.
- implicit `.env` loading.

## CLI Workflow

The MVP exposes three commands:

- `sqlcomp init` creates a `sqlcomp.config.json` template and refuses to overwrite
  an existing config file.
- `sqlcomp check` runs the full compile pipeline, including MySQL metadata lookup,
  but does not write generated files.
- `sqlcomp compile` writes generated TypeScript SQL builder files.

`sqlcomp compile --clean` removes stale managed generated files that no longer
correspond to an input SQL file. Normal `compile` does not remove stale files.

When `--config` is not provided, `sqlcomp` searches from the current working
directory upward for `sqlcomp.config.json`.

## Configuration

`sqlcomp.config.json` is parsed as JSON with comments and trailing commas allowed.
The MVP configuration shape is:

```jsonc
{
  "source": {
    "include": ["sql/**/*.sql"],
    "exclude": [],
  },
  "output": {
    "dir": "src/generated/sqlcomp",
  },
  "database": {
    "dialect": "mysql",
    "urlEnv": "DATABASE_URL",
  },
  "target": {
    "language": "typescript",
  },
}
```

For the MVP, `source.include`, `output.dir`, `database.dialect`,
`database.urlEnv`, and `target.language` are required. `source.exclude` is
optional.

Configuration paths are resolved relative to the directory containing
`sqlcomp.config.json`. Generated TypeScript preserves each input SQL path relative
to that same directory.

The database connection URL is read from the process environment using
`database.urlEnv`. The CLI does not implicitly load `.env` files.

## Query Blocks

Each query starts with a `type: query` annotation. The SQL body continues until the
next `type: query` annotation or the end of the file.

Each query body must contain exactly one `SELECT` statement and must end with `;`.

An included `.sql` file that contains SQL text but no `@sqlcomp` query annotation
emits a warning so users can tell the file was ignored by the query compiler. Empty
files and comment-only files do not warn.

```sql
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id, name FROM users;

/* @sqlcomp
{
  type: query
  id: findLatestUser
}
*/
SELECT id, name FROM users ORDER BY id DESC LIMIT 1;
```

## Query Metadata

`id` is required. It is never inferred from the file name, SQL text, or output path.

Valid IDs must match:

```text
^[A-Za-z_][A-Za-z0-9_]*$
```

`cardinality` is optional.

MVP cardinality inference:

- `SELECT ... LIMIT 1` infers `one`.
- other `SELECT` statements infer `many`.
- `one` means `Row | null`.
- `many` means `Row[]`.
- `exec` is reserved for future non-SELECT support and must be rejected in the MVP.

An explicit `cardinality` value overrides inference when the value is supported by
the MVP.

Query IDs must be unique across the full compile run, not only within a single SQL
file.

## Generated TypeScript

Generated TypeScript uses the query `id` exactly as written. It does not convert
between camelCase, PascalCase, or snake_case.

For `id: listUsers`, generated symbols are:

- `listUsers`
- `listUsers_Input`
- `listUsers_Row`
- `listUsers_Output`

Generated functions return SQL builder data:

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

The `input` parameter exists to keep the public shape stable for future `Param`
support. In the MVP, generated functions should name the unused parameter `_input`
so projects with `noUnusedParameters` enabled can type-check generated code. Query
inputs are always `Record<string, never>`, and `params` is always an empty readonly
tuple.

Generated SQL must be emitted as a valid JavaScript string literal. The generator
must escape SQL text instead of copying raw SQL into an unescaped template literal,
because valid MySQL SQL may contain backtick identifiers or `${...}` text that would
otherwise break generated TypeScript.

Generated files include a generated-code header. `compile` treats `output.dir` as a
generated area and overwrites same-path files.

## Acceptance Scenarios

The implementation should cover these scenarios:

- multiple queries in one `.sql` file are generated into one corresponding `.ts`
  file.
- multiple `.sql` files preserve their config-file-relative paths under
  `output.dir`.
- duplicate query IDs are rejected.
- invalid query IDs are rejected.
- `check` performs a database-backed dry run without writing files.
- `compile --clean` removes stale managed generated files.
- non-`SELECT` statements are rejected.
- query blocks with multiple SQL statements are rejected.
- `LIMIT 1` infers `one`.
- ordinary `SELECT` infers `many`.
- explicit `cardinality` overrides inference.
- `cardinality: exec` is rejected in the MVP.
- included SQL files with SQL text but no `@sqlcomp` query annotations warn.
- MySQL nullable metadata maps to `T | null`.
- unknown nullability maps to `T | null`.

See also:

- [ADR 0006: Define MVP CLI, Config, and Generation Workflow](./adr/0006-define-mvp-cli-config-and-generation-workflow.md)
- [ADR 0004: Limit the MVP to Query-only SELECT support](./adr/0004-limit-mvp-to-query-only-select.md)
