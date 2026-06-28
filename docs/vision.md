# Vision

`sqlay` is SQL Inlay.

It is a CLI tool for writing plain SQL files while gaining compile-time type safety
and predictable SQL composition for statically typed languages.

## Tagline

Write Pure SQL. Feel Type Safety. No Magic.

## Core Philosophy

### 2-Way SQL

SQL files should remain usable as SQL. A developer should be able to copy or open a
statement in a normal database tool and understand what will run without first
understanding generated code.

`@sqlay` metadata is carried in SQL comments. The metadata may guide compilation,
but it must not require the SQL text to become a custom DSL.

### Explicit Design

`sqlay` should prefer explicit user intent over implicit compiler behavior.

The compiler must not silently rewrite SQL structure, replace table aliases, infer
public API names, or apply language-specific naming conventions. If a name matters
to generated code, the user should choose a suitable name in the source SQL.

### Static Type Safety

Generated code should represent database metadata in the target language's type
system where sqlay can do so responsibly.

For SELECT query builders, the current supported target generates TypeScript result
row types and typed input values for supported `Param` markers. When result
metadata is uncertain, generated types should be conservative rather than
overconfident. For example, unknown nullability should be treated as nullable.
When schema-backed metadata is precise, generated types may be more specific, such
as inline literal unions for MySQL `ENUM` values.

Projects may explicitly override generated TypeScript type annotations when the
default conservative mapping does not match their domain model or driver
configuration. Those overrides are still static type annotations. They must not
imply runtime result parsing, input validation, generated driver configuration, or
SQL execution behavior.

For mutation builders, sqlay should generate typed inputs and parameter arrays, but
it should not claim to know driver execution results such as affected row counts,
generated IDs, or final row state.

### Minimal Runtime Surface

Generated code should have a small runtime surface. The current TypeScript target
generates SQL builder functions that return SQL text and parameters. It does not
execute statements or require a database driver in generated TypeScript code.

Driver usage belongs in user code and examples, not in generated builders.

### Flat Result Mapping

SELECT rows are mapped directly to language-level object types. `sqlay` does not
generate nested object graphs, identity maps, change tracking, migrations, or
ORM-style models.

## Current Boundaries

The current implementation supports query metadata, result type extraction, SELECT
value binding through inline `Param` markers, TypeScript SQL builder output, and
initial `Slot`/`Fragment` validation with generated Slot input types and runtime SQL
branch builders.

[ADR 0010](./adr/0010-define-initial-mysql-mutation-builder-support.md) defines the
accepted direction for MySQL mutation builders covering `INSERT`, `UPDATE`,
`DELETE`, and `REPLACE`. Mutation support remains a SQL builder feature: execution,
transaction management, and driver-specific result types stay outside generated
code.

[ADR 0011](./adr/0011-define-repeat-for-variable-length-sql-repetition.md) defines
the accepted direction for `Repeat`, which repeats one SQL list item template for
dynamic `IN` lists and bulk `VALUES` rows while keeping generated code as SQL
builder output only.

[ADR 0012](./adr/0012-define-configurable-typescript-type-mapping-overrides.md)
defines the accepted direction for configurable TypeScript type annotation
overrides, including schema-backed MySQL enum literal unions and explicit project
type mappings.

Additional SQL dialects and additional target generators require separate design
decisions before implementation.

`Slot` design uses `targets: [...]` rather than a single `target`, so exclusive
choices and single choices share one representation.
