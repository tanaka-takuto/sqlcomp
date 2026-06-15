# Architecture

`sqlcomp` is implemented as a Rust CLI with a small set of components connected by
explicit intermediate representations. The components are named by responsibility so
their database and target-language dependencies stay visible.

The authoritative product rules live in [Vision](./vision.md), and the current MVP
boundary lives in [MVP](./mvp.md).

## Crate Layout

`sqlcomp` is a Cargo workspace. Component boundaries are represented as separate
workspace crates, not only as Rust modules, so dependency direction is enforced by
`Cargo.toml` path dependencies.

The root `src/` directory belongs only to the final `sqlcomp` binary package.
Reusable implementation crates live under `crates/`, which is the conventional
workspace layout for multiple Rust packages in one repository.

Runtime flow and dependency direction are intentionally different. Runtime flow
moves from intake toward generation, but crate dependencies point inward. Inner
crates never depend on outer crates.

```text
sqlcomp binary crate
  -> sqlcomp-cli

sqlcomp-cli
  -> sqlcomp-app
  -> sqlcomp-adapters

sqlcomp-adapters
  -> sqlcomp-app
  -> sqlcomp-core

sqlcomp-app
  -> sqlcomp-core

sqlcomp-core
  -> no sqlcomp-* dependencies
```

Only `sqlcomp-cli` may depend on both `sqlcomp-app` and `sqlcomp-adapters`.
`sqlcomp-cli` is the composition root: it wires concrete adapters into application
ports. `sqlcomp-adapters` groups infrastructure adapters as modules such as
`config_jsonc`, `source_fs`, `dialect_mysql`, `metadata_mysql_sqlx`,
`target_typescript`, and `output_fs`. Adapter modules implement ports from
`sqlcomp-app` and exchange only `sqlcomp-core` types. `sqlcomp-app` owns use cases
and port traits. `sqlcomp-core` owns shared domain vocabulary and language-neutral
IR. A new dependency edge between workspace crates is an architecture decision, not
an incidental import.

## Component Flow

```text
CLI Driver
  -> Config Loader
  -> Compilation Plan

Source Intake
  -> RawQuery

Dialect Analyzer
  RawQuery + dialect rules
  -> AnalyzedQuery

Metadata Provider
  RawQuery.sql + database connection
  -> DbQueryMetadata

Application Use Case + Core IR
  RawQuery + AnalyzedQuery + DbQueryMetadata
  -> CompiledQuery

Target Generator
  CompiledQuery
  -> generated files
```

This structure avoids a direct `database dialect x target language` implementation
matrix. Database-specific logic maps database behavior into the Core IR. Target
generators map the Core IR into language-specific code.

## Diagnostics and Errors

Components that can fail with user-facing MVP errors return shared diagnostic
primitives from `sqlcomp-core` instead of formatting final CLI output themselves.
Diagnostics carry a human-readable message and may include file path and one-based
source location context when that information is available.

The CLI remains responsible for converting diagnostic reports into stderr output
and process exit codes. Application services and adapters should return structured
diagnostics to the CLI boundary.

## CLI Driver

The CLI Driver owns command selection, configuration discovery, process environment
access, and user-facing diagnostics. It should not parse SQL or generate
TypeScript directly.

The CLI crate is also the composition root. It may depend on `sqlcomp-app`,
`sqlcomp-core`, and all concrete adapter crates. No inner crate may depend on the
CLI crate.

For the MVP, the command surface is:

- `init` writes a starter `sqlcomp.config.json`.
- `check` runs the full compile pipeline without writing generated files.
- `compile` writes generated TypeScript files.

`check` and `compile` use the same analysis and generation pipeline so CI and local
generation validate the same behavior.

## Config Loader

Config Loader resolves the project configuration before Source Intake runs.

Responsibilities:

- find `sqlcomp.config.json` from the current working directory upward when
  `--config` is not provided.
- parse JSON with comments and trailing commas allowed.
- validate the supported MVP values for source, output, database, and target
  settings.
- resolve source and output paths relative to the configuration file directory.
- read the database URL from the process environment using `database.urlEnv`.

The CLI does not implicitly load `.env` files in the MVP.

## Compilation Plan

The Compilation Plan is the resolved work order produced from configuration. It is
not a semantic SQL representation.

Responsibilities:

- expand `source.include` and `source.exclude` into the input SQL file set.
- keep each input file path relative to the configuration file directory.
- carry the resolved output directory.
- carry the database URL and target selection for downstream components.

## Source Intake

Source Intake reads SQL files and extracts sqlcomp source units. It does not decide
whether the SQL is valid MySQL, PostgreSQL, or another dialect.

Responsibilities:

- read `.sql` files.
- find `@sqlcomp` comments.
- parse Hjson metadata payloads.
- split files into raw query blocks.
- preserve each query block's raw SQL string.
- collect independent source-intake diagnostics across discovered SQL files before
  returning failure.

Source Intake is not fully independent from SQL syntax because it must scan SQL
comments and avoid corrupting string literals or comment-like text. However, it
should avoid database semantic decisions. It should produce `RawQuery` values for the
configured dialect analyzer to interpret.

The canonical query annotation form is:

```sql
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id, name FROM users;
```

For the MVP:

- `type: query` is required.
- `id` is required and is never inferred.
- `id` must match `^[A-Za-z_][A-Za-z0-9_]*$`.
- `id` must be unique across the full compile run.
- `cardinality` is optional and may override compiler inference.
- one SQL file may contain multiple query annotations.

For post-MVP SELECT `Param` intake, `type: query` remains the only annotation that
starts a new query block. Inline `type: param` and `type: paramEnd` annotations are
recognized inside query bodies as defined by
[ADR 0008](./adr/0008-define-select-param-support.md).

## Dialect Analyzer

The Dialect Analyzer interprets a `RawQuery` as SQL for one configured database
dialect.

For the MVP, the only dialect analyzer is MySQL 8.x.

Responsibilities:

- parse the raw SQL according to dialect rules.
- reject unsupported statement forms.
- verify that each MVP query block contains exactly one `SELECT` statement.
- infer dialect-dependent query facts such as `LIMIT 1` cardinality.
- produce `AnalyzedQuery` without target-language concerns.

Future PostgreSQL or SQLite support should add new dialect analyzers rather than
branching inside target generators.

## Metadata Provider

The Metadata Provider obtains database metadata for an analyzed query.

For the MVP, the provider connects to MySQL 8.x and derives result column metadata
without executing user data queries. The default Rust database client is `sqlx`,
pending implementation validation. If `sqlx` cannot expose the required MySQL
statement and column metadata, the project should record a follow-up ADR before
changing the client.

Responsibilities:

- connect to the configured database.
- describe a query without fetching user data.
- return database-native column names, database types, and nullability metadata.

See also:

- [ADR 0001: Use MySQL 8.x as the MVP dialect](./adr/0001-use-mysql-8-for-mvp.md)
- [ADR 0003: Use Hjson `@sqlcomp` comments](./adr/0003-use-hjson-sqlcomp-comments.md)

## Application Use Cases and Ports

Application use cases coordinate the MVP workflow and own the port traits that
adapters implement.

Responsibilities:

- define ports such as config loading, source reading, dialect analysis, metadata
  lookup, target generation, and generated-file writing.
- coordinate `init`, `check`, and `compile` workflows.
- depend only on `sqlcomp-core`.
- avoid filesystem, database, SQL parser, and TypeScript formatting implementation
  details.

## Compilation Core

Compilation Core is the innermost crate. It owns shared domain vocabulary and
language-neutral Core IR. It must not depend on source intake, dialect analyzers,
metadata providers, target generators, or the CLI.

IR means intermediate representation: an internal data structure that is no longer
raw SQL input, but is not yet TypeScript, Go, Rust, or any other generated language.

Example Core IR shape:

```rust
struct CompiledQuery {
    id: QueryId,
    sql: String,
    cardinality: Cardinality,
    input: Vec<InputField>,
    row: Vec<ResultColumn>,
}

struct ResultColumn {
    name: String,
    ty: CoreType,
    nullable: bool,
}

enum CoreType {
    Bool,
    Int32,
    Int64,
    Float64,
    Decimal,
    String,
    Bytes,
    Date,
    Time,
    DateTime,
    Json,
    Unknown,
}
```

Database-specific type mapping should stop at Core IR:

```text
MySQL BIGINT -> CoreType::Int64
PostgreSQL int8 -> CoreType::Int64
```

Target-language type mapping should start from Core IR:

```text
CoreType::Int64 -> TypeScript string
CoreType::Int64 -> Go int64
```

This keeps MySQL-to-TypeScript, PostgreSQL-to-TypeScript, MySQL-to-Go, and
PostgreSQL-to-Go from becoming separate hard-coded paths.

Core metadata should be conservative:

- database nullability metadata is used when available.
- unknown nullability maps to nullable output.
- precision-sensitive types such as `BIGINT`, `DECIMAL`, and date/time values should
  avoid lossy JavaScript conversions in the MVP target generator.

## Target Generator

Target Generators convert Core IR into generated files for a target language. They
should not parse or reinterpret database-specific SQL syntax. The SQL text inside a
generated file may be MySQL or another dialect, but the generator treats that SQL as
validated text carried by the Core IR.

The MVP target generator emits TypeScript SQL builder code. Generated code returns
SQL text and parameter arrays, not database execution behavior.

Generated TypeScript is emitted per SQL file while preserving the input path
relative to the directory containing `sqlcomp.config.json`. If one SQL file
contains multiple queries, the corresponding TypeScript file contains multiple
generated query functions and type aliases.

Generated names are not case-converted. The query `id` is used exactly as written,
with fixed suffixes for generated TypeScript types:

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

Generated SQL must be emitted as a valid JavaScript string literal. The TypeScript
target generator should escape the SQL text rather than embedding raw SQL in an
unescaped template literal, because MySQL backtick identifiers and SQL text
containing `${...}` must not break generated TypeScript. MVP examples use ordinary
double-quoted string literals; multiline SQL may use any representation that is
semantically equivalent after JavaScript string escaping.

Generated files include a generated-code header. The MVP treats the configured
output directory as generated output and overwrites same-path files during
`compile`. Stale generated files are removed only when `compile --clean` is used.

## Development and Integration Checks

The project should keep local and CI checks aligned. Rust formatting, linting, and
unit tests remain the external-service-free baseline checks. MySQL-backed checks are
separate because they require a running MySQL 8.x database and prefix-scoped schema
reset.

Examples and fixtures have different responsibilities. `examples/` contains
user-facing sample projects with generated TypeScript output that is actual compiler
output. `fixtures/` contains implementation-focused test inputs and expected
artifacts for coverage, edge cases, and diagnostics. DB-backed generated-output
checks should regenerate examples and fixtures in temporary directories, compare the
generated output byte for byte with committed expected artifacts, and type-check the
generated TypeScript. These checks should not use Git working-tree diffs as their
oracle.

Rust tests should follow the conventional crate layout:

- unit tests live inside the module they test, usually in a `#[cfg(test)] mod tests`
  block near the implementation inside the owning crate.
- integration tests live outside the crate source tree under that package's
  `tests/` directory and exercise public crate APIs the way an external caller
  would.

This placement is intentional. Component-local behavior should be tested from
inside the component module so private helpers can stay private. Cross-component,
CLI, generated-output, filesystem, and database-backed behavior should be tested from
the appropriate package-level `tests/` directory so the test boundary matches the
public library or binary behavior.

Generated TypeScript should be type-checked in CI with `tsc --noEmit` once the
generator exists. This verifies that generated builders are usable in ordinary
TypeScript projects without adding runtime dependencies.

See also:

- [ADR 0006: Define MVP CLI, Config, and Generation Workflow](./adr/0006-define-mvp-cli-config-and-generation-workflow.md)
- [ADR 0002: Use TypeScript SQL builders as the first target generator](./adr/0002-use-typescript-target-generator-first.md)
- [ADR 0005: Do not automatically transform generated names](./adr/0005-do-not-transform-generated-names.md)
- [ADR 0007: Use examples and fixtures as generated E2E artifacts](./adr/0007-use-examples-and-fixtures-as-generated-e2e-artifacts.md)
