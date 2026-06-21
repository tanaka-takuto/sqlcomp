# Vision

`sqlay` is SQL Inlay.

It is a CLI tool for writing plain SQL files while gaining compile-time type safety
and predictable query composition for statically typed languages.

## Tagline

Write Pure SQL. Feel Type Safety. No Magic.

## Core Philosophy

### 2-Way SQL

SQL files should remain usable as SQL. A developer should be able to copy or open a
query in a normal database tool and understand what will run without first
understanding generated code.

`@sqlay` metadata is carried in SQL comments. The metadata may guide compilation,
but it must not require the SQL text to become a custom DSL.

### Explicit Design

`sqlay` should prefer explicit user intent over implicit compiler behavior.

The compiler must not silently rewrite SQL structure, replace table aliases, infer
public API names, or apply language-specific naming conventions. If a name matters
to generated code, the user should choose a suitable name in the source query.

### Static Type Safety

Generated code should represent database result metadata in the target language's
type system. The current supported target generates TypeScript types for MySQL
`SELECT` result rows and typed input values for supported `Param` markers.

When metadata is uncertain, generated types should be conservative rather than
overconfident. For example, unknown nullability should be treated as nullable.

### Minimal Runtime Surface

Generated code should have a small runtime surface. The current TypeScript target
generates SQL builder functions that return SQL text and parameters. It does not
execute queries or require a database driver in generated TypeScript code.

### Flat Result Mapping

Rows are mapped directly to language-level object types. `sqlay` does not
generate nested object graphs or ORM-style models.

## Current Boundaries

The current implementation supports query metadata, result type extraction, SELECT
value binding through inline `Param` markers, TypeScript SQL builder output, and
initial `Slot`/`Fragment` validation with generated Slot input types and runtime SQL
branch builders.

Non-SELECT statements, generated database execution functions, additional SQL
dialects, and additional target generators require separate design decisions before
implementation.

`Slot` design uses `targets: [...]` rather than a single `target`, so exclusive
choices and single choices share one representation.
