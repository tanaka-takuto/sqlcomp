# Current Scope

This document is the active entrypoint for what `sqlcomp` currently supports and
where near-term work should point. The original MVP remains documented in
[Initial MVP Baseline](./mvp.md) and the accepted ADRs.

## Supported Capability Set

`sqlcomp` currently supports:

- SQL source files annotated with `@sqlcomp` Hjson block comments.
- `type: query` annotations with an explicit `id` and optional
  `cardinality: one | many`.
- MySQL 8.x analysis for query blocks that contain exactly one `SELECT` statement.
- TypeScript SQL builder generation that preserves SQL source paths under
  `output.dir`.
- `sqlcomp.config.json` project configuration, discovered from the working
  directory upward when `--config` is omitted.
- `init`, `check`, and `compile` CLI commands.
- SELECT value binding with paired inline `Param` markers as defined by
  [ADR 0008](./adr/0008-define-select-param-support.md).

Generated TypeScript builders return SQL text and parameter arrays. They do not
execute queries and do not depend on a database driver.

## Param Support

Inline `Param` markers wrap a sample SQL expression so source queries remain
directly readable and executable in database tools:

```sql
SELECT u.id, u.email
FROM users AS u
WHERE u.email = /* @sqlcomp { type: param id: email } */
  'test@example.test'
  /* @sqlcomp { type: paramEnd } */;
```

For compilation, each Param range is replaced with one MySQL `?` placeholder. Input
types are inferred from qualified direct MySQL column context such as `u.email` when
possible, or from an inline `valueType` override. `nullable: true` marks nullable
input values. Repeated Param IDs are supported when all occurrences agree on type
and nullability.

## Near-Term Direction

The near-term direction is to stabilize the current SELECT builder workflow:

- keep contributor and user documentation aligned with the post-MVP scope.
- keep diagnostics and CLI help explicit about supported dialects, statement kinds,
  target languages, and Param syntax.
- maintain examples, fixtures, and generated TypeScript artifacts as executable
  coverage for the supported workflow.

Larger expansions should be captured in ADRs before implementation.

## Defining ADRs

The current scope is defined by these accepted ADRs:

- [ADR 0001: Use MySQL 8.x as the MVP Dialect](./adr/0001-use-mysql-8-for-mvp.md)
- [ADR 0002: Use TypeScript SQL Builders as the First Target Generator](./adr/0002-use-typescript-target-generator-first.md)
- [ADR 0003: Use Hjson `@sqlcomp` Comments](./adr/0003-use-hjson-sqlcomp-comments.md)
- [ADR 0004: Limit the MVP to Query-only SELECT Support](./adr/0004-limit-mvp-to-query-only-select.md)
- [ADR 0005: Do Not Automatically Transform Generated Names](./adr/0005-do-not-transform-generated-names.md)
- [ADR 0006: Define MVP CLI, Config, and Generation Workflow](./adr/0006-define-mvp-cli-config-and-generation-workflow.md)
- [ADR 0007: Use Examples and Fixtures as Generated E2E Artifacts](./adr/0007-use-examples-and-fixtures-as-generated-e2e-artifacts.md)
- [ADR 0008: Define SELECT Param Support](./adr/0008-define-select-param-support.md)

## Out Of Scope

The following remain intentionally unsupported:

- `Slot` and `Fragment` dynamic SQL composition.
- optional input properties that would require SQL structure changes.
- `INSERT`, `UPDATE`, `DELETE`, DDL, `CALL`, and other non-SELECT statements.
- multi-statement query blocks.
- generated database execution functions.
- non-MySQL dialects.
- non-TypeScript target generators.
- automatic naming transformation.
- implicit `.env` loading.
