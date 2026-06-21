# ADR 0007: Use Examples and Fixtures as Generated E2E Artifacts

## Status

Accepted

## Context

The MVP needs examples that users can read to understand practical `sqlay` usage.
Those examples should not be hand-written approximations of generated output. If an
example contains generated TypeScript, that TypeScript should be the actual output of
the compiler for the example SQL and database schema.

The project also needs fixtures that maximize implementation coverage. These
fixtures may be less natural as user-facing examples because their purpose is to
exercise edge cases, type mapping, nullability behavior, invalid inputs, and
diagnostics.

Using `git diff` as the E2E oracle would couple test success to the caller's Git
working tree. E2E checks should instead compare the generated files for the current
run against committed expected artifacts directly.

## Decision

The repository separates examples from fixtures by audience and purpose:

- `examples/` contains user-facing sample projects. Examples prioritize realistic
  use cases, readable SQL, natural naming, and generated output that users can
  inspect.
- `fixtures/` contains test inputs and expected artifacts. Fixtures prioritize
  coverage, edge cases, and precise regression detection over readability as a
  sample application.

The initial user-facing example project is `examples/bookstore/`. It uses
`bookstore_` as the database object prefix and keeps generated TypeScript under the
example's configured `output.dir`. The example output directory is `generated/`, so
SQL files under `sql/**/*.sql` generate files such as `generated/sql/books.ts`.

SQL fixtures use `fixture_` as the database object prefix. SQL fixture source files
live under `fixtures/sql/valid/` and `fixtures/sql/invalid/`, with
`fixtures/sql/sqlay.valid.config.json` and
`fixtures/sql/sqlay.invalid.config.json` making each side executable as an
isolated sqlay project. The primary positive MySQL fixture should be named by
purpose, such as `fixtures/sql/valid/type_metadata_matrix.sql`, rather than by a
broad name such as `metadata.sql`. Practical business-style reads belong in
examples, not SQL fixtures.

Generated TypeScript expected artifacts are committed for both examples and
fixtures. They are treated as compiler output, not as hand-authored sample code.

DB-backed generated-output checks use this model:

1. Copy the example or fixture project into a temporary directory.
2. Reset the relevant database objects by loading idempotent schema and seed files.
3. Run `sqlay compile` against the temporary copy.
4. Compare the temporary generated output with the committed expected generated
   output byte for byte.
5. Run `tsc --noEmit` for the generated TypeScript surface.

Checks must not use `git diff` as the comparison mechanism. A direct file or
directory byte comparison is the E2E oracle.

Schema files used by examples and fixtures are idempotent reset scripts for their
own prefixes. They should drop and recreate only their owned objects, such as
`bookstore_` or `fixture_` tables. The MySQL development service should start a
database only; fixture or example data loading belongs to the relevant check script.

The check scripts should be named by scope:

- `script/check-baseline.sh` runs external-service-free baseline checks.
- `script/check-examples.sh` runs DB-backed example compile, generated comparison,
  and TypeScript checks.
- `script/check-mysql-fixtures.sh` runs DB-backed MySQL fixture checks, generated
  comparison, and fixture TypeScript checks.

TypeScript package scripts should also be named by scope:

- `typecheck:examples` type-checks user-facing generated example artifacts.
- `typecheck:fixtures` type-checks generated fixture artifacts.

## Consequences

Examples become trustworthy demonstrations because their generated files are actual
compiler output and are verified by E2E checks.

Fixtures can grow into a high-coverage matrix without making examples hard to read.
The existing business-style MySQL fixture should be removed once the bookstore
example covers practical user scenarios.

Generated TypeScript changes become explicit public-output changes. Byte-for-byte
comparison intentionally catches formatting, escaping, path mapping, and type-shape
changes.

The baseline check no longer claims to be "all" checks. DB-backed examples and
fixtures remain separate checks because they require MySQL and schema reset.

Local and CI database state is more reproducible because each DB-backed check owns
its prefix-scoped reset instead of relying on container initialization side effects.
