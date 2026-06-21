# Current Scope

This document is the active entrypoint for what `sqlay` currently supports and
where near-term work should point. The original MVP remains documented in
[Initial MVP Baseline](./mvp.md) and the accepted ADRs.

## Supported Capability Set

`sqlay` currently supports:

- SQL source files annotated with `@sqlay` Hjson block comments.
- `type: query` annotations with an explicit `id` and optional
  `cardinality: one | many`.
- MySQL 8.x analysis for query blocks that contain exactly one `SELECT` statement.
- TypeScript SQL builder generation that preserves SQL source paths under
  `output.dir`.
- `sqlay.config.json` project configuration, discovered from the working
  directory upward when `--config` is omitted. For `check` and `compile`, an
  explicit config path is accepted before the command or as a command option.
- `source.include` and `source.exclude` paths resolved from the configuration
  directory, with matched SQL files required to stay inside that directory.
- `init`, `check`, and `compile` CLI commands.
- SELECT value binding with paired inline `Param` markers as defined by
  [ADR 0008](./adr/0008-define-select-param-support.md).
- Initial SELECT `Slot`/`Fragment` composition as defined by
  [ADR 0009](./adr/0009-define-initial-select-slot-fragment-support.md), including
  validation variants, Slot input types, and runtime TypeScript SQL branch builders.

Generated TypeScript builders return SQL text and parameter arrays. They do not
execute queries and do not depend on a database driver.

## Param Support

Inline `Param` markers wrap a sample SQL expression so source queries remain
directly readable and executable in database tools:

```sql
SELECT u.id, u.email
FROM users AS u
WHERE u.email = /* @sqlay { type: param id: email } */
  'test@example.test'
  /* @sqlay { type: paramEnd } */;
```

For compilation, each Param range is replaced with one MySQL `?` placeholder. Input
types are inferred from qualified direct MySQL column context such as `u.email` when
possible, or from an inline `valueType` override. `nullable: true` marks nullable
input values. Repeated Param IDs are supported when all occurrences agree on type
and nullability.

`valueType` uses lower-case sqlay CoreType names, not TypeScript union syntax. For
a nullable datetime input, write `valueType: datetime` with `nullable: true`; the
generated TypeScript input field is `string | null` because datetime values map to
strings:

```sql
WHERE u.created_at < /* @sqlay { type: param id: createdBefore valueType: datetime nullable: true } */
  '2026-01-01 00:00:00'
  /* @sqlay { type: paramEnd } */
```

```ts
export type listUsers_Input = {
  createdBefore: string | null;
};
```

Optional direct Param input properties are not currently supported because omitting
a direct Param input would require changing the SQL structure. Current query authors
should either use a nullable sentinel pattern such as `param IS NULL OR column =
param`, write separate queries for distinct shapes, or use Slot/Fragment selection
for supported dynamic SQL composition slices.

## Near-Term Direction

The near-term direction is to stabilize the current SELECT builder workflow:

- keep contributor and user documentation aligned with the post-MVP scope.
- keep diagnostics and CLI help explicit about supported dialects, statement kinds,
  target languages, and Param syntax.
- maintain examples, fixtures, and generated TypeScript artifacts as executable
  coverage for the supported workflow.

Larger expansions should be captured in ADRs before implementation.

Configuration placement matters for generated path preservation. If SQL files live
in a sibling directory such as `sql/` next to `configs/`, place
`sqlay.config.json` at the common project root instead of including `../sql/**`
from a nested config file.

The initial SELECT `Slot`/`Fragment` design is captured in
[ADR 0009](./adr/0009-define-initial-select-slot-fragment-support.md). The current
implementation has started landing validation slices for that ADR: query-local
Slots and global Fragments can be resolved into concrete validation variants during
`check` and `compile`; variants use source-authored whitespace without compiler
normalization and preserve expanded-SQL Param ordering. Validation rejects queries
that would produce more than 256 variants, unknown Slot targets, or duplicate Slot
targets before dialect analysis. Validation also rejects expanded variants whose
effective cardinality, after any explicit query metadata override is applied, or
result row shape differs from the all-slots-unselected base variant, and repeated
Slot occurrences whose selected Fragment Param type or nullability conflicts.
TypeScript generation now includes Slot input types with optional `$fragment`
discriminated branch objects, nesting Fragment Params inside the selected Slot
branch, and runtime builders that assemble selected SQL segments and Params in
expanded SQL order. CLI success summaries report Fragment, unique Slot, and
validated variant counts.

## Defining ADRs

The current scope is defined by these accepted ADRs:

- [ADR 0001: Use MySQL 8.x as the MVP Dialect](./adr/0001-use-mysql-8-for-mvp.md)
- [ADR 0002: Use TypeScript SQL Builders as the First Target Generator](./adr/0002-use-typescript-target-generator-first.md)
- [ADR 0003: Use Hjson `@sqlay` Comments](./adr/0003-use-hjson-sqlay-comments.md)
- [ADR 0004: Limit the MVP to Query-only SELECT Support](./adr/0004-limit-mvp-to-query-only-select.md)
- [ADR 0005: Do Not Automatically Transform Generated Names](./adr/0005-do-not-transform-generated-names.md)
- [ADR 0006: Define MVP CLI, Config, and Generation Workflow](./adr/0006-define-mvp-cli-config-and-generation-workflow.md)
- [ADR 0007: Use Examples and Fixtures as Generated E2E Artifacts](./adr/0007-use-examples-and-fixtures-as-generated-e2e-artifacts.md)
- [ADR 0008: Define SELECT Param Support](./adr/0008-define-select-param-support.md)
- [ADR 0009: Define Initial SELECT Slot/Fragment Support](./adr/0009-define-initial-select-slot-fragment-support.md)

## Out Of Scope

The following remain intentionally unsupported:

- `Slot` and `Fragment` features outside the initial ADR 0009 design, including
  required slots, default fragments, multi-select slots, fragment-local slots,
  fragment include or alias, and result-shape-changing variants.
- optional direct Param input properties that would require SQL structure changes.
- `INSERT`, `UPDATE`, `DELETE`, DDL, `CALL`, and other non-SELECT statements.
- multi-statement query blocks.
- generated database execution functions.
- non-MySQL dialects.
- non-TypeScript target generators.
- automatic naming transformation.
- implicit `.env` loading.
