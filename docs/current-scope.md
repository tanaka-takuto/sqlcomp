# Current Scope

This document is the active entrypoint for what `sqlay` currently supports and
where near-term work should point. The original MVP remains documented in
[Initial MVP Baseline](./mvp.md) and the accepted ADRs.

## Supported Capability Set

`sqlay` currently supports:

- SQL source files annotated with `@sqlay` Hjson block comments.
- `type: query` annotations with an explicit `id` and optional
  `cardinality: one | many`.
- `type: mutation` annotations with an explicit `id`.
- MySQL 8.x analysis for query blocks that contain exactly one `SELECT` statement.
- MySQL 8.x analysis for mutation blocks that contain exactly one supported
  `INSERT`, `UPDATE`, `DELETE`, or `REPLACE` statement as defined by
  [ADR 0010](./adr/0010-define-initial-mysql-mutation-builder-support.md).
- TypeScript SQL builder generation that preserves SQL source paths under
  `output.dir`.
- `sqlay.config.json` project configuration, discovered from the working
  directory upward when `--config` is omitted. For `check` and `compile`, an
  explicit config path is accepted before the command or as a command option.
- `source.include` and `source.exclude` paths resolved from the configuration
  directory, with matched SQL files required to stay inside that directory.
- `init`, `check`, and `compile` CLI commands.
- Empty `source.include` matches are reported as warnings after applying
  `source.exclude`; `check --fail-on-empty` and `compile --fail-on-empty` promote
  that condition to a failing diagnostic before generated files are written or
  cleaned.
- SELECT value binding with paired inline `Param` markers as defined by
  [ADR 0008](./adr/0008-define-select-param-support.md).
- Initial SELECT `Slot`/`Fragment` composition as defined by
  [ADR 0009](./adr/0009-define-initial-select-slot-fragment-support.md), including
  validation variants, Slot input types, and runtime TypeScript SQL branch builders.

Generated TypeScript builders return SQL text and parameter arrays. They do not
execute queries or mutations and do not depend on a database driver.

## Param Support

Inline `Param` markers wrap a sample SQL expression so source SQL remains directly
readable and executable in database tools:

```sql
SELECT u.id, u.email
FROM users AS u
WHERE u.email = /* @sqlay { type: param id: email } */
  'test@example.test'
  /* @sqlay { type: paramEnd } */;
```

For compilation, each Param range is replaced with one MySQL `?` placeholder. Input
types are inferred from supported direct MySQL column contexts when possible, or
from an inline `valueType` override. `nullable: true` marks nullable input values.
Repeated Param IDs are supported when all occurrences agree on type and
nullability.

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
a direct Param input would require changing the SQL structure. Current authors
should either use a nullable sentinel pattern such as `param IS NULL OR column =
param`, write separate builders for distinct shapes, or use Slot/Fragment
selection for supported dynamic SQL composition slices.

## Accepted Mutation Direction

[ADR 0010](./adr/0010-define-initial-mysql-mutation-builder-support.md) defines the
accepted direction for initial MySQL mutation builders. The mutation feature is a
typed SQL builder capability, not a generated database execution layer.

The accepted mutation source unit is:

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

Initial mutation support targets MySQL `INSERT`, `UPDATE`, `DELETE`, and `REPLACE`
builders. Generated mutation builders return SQL text and params only. They do not
generate result row types, output types, execution functions, transaction helpers,
or driver-specific result wrappers.

See [Mutation Execution with mysql2](./mutation-execution.md) for user-facing
execution examples that keep generated builders driver-independent.

Mutation `Param` inference is schema-backed and limited to supported direct column
contexts such as `INSERT` column lists, `SET column = param`, and qualified
predicate columns. Mutation SQL is never executed during `check` or `compile`.

Mutation Slot/Fragment composition uses the same optional single-select Slot model
as SELECT queries, but validation is mutation-specific: every variant must remain a
supported single mutation statement with the same statement kind as the base
variant.

## Accepted Repeat Direction

[ADR 0011](./adr/0011-define-repeat-for-variable-length-sql-repetition.md) defines
the accepted direction for `Repeat`, a future variable-length SQL repetition
feature for dynamic `IN` lists and bulk `VALUES` rows.

Repeat uses paired inline `repeat` and `repeatEnd` markers around one list item
template. The surrounding list syntax remains ordinary SQL:

```sql
AND u.id IN (
  /* @sqlay { type: repeat id: ids separator: "," } */
  /* @sqlay { type: param id: id valueType: int64 } */
  1
  /* @sqlay { type: paramEnd } */
  /* @sqlay { type: repeatEnd } */
)
```

Repeat input is always a non-empty readonly array of item objects. Single-Param
items still use object items, not scalar arrays. Repeat arrays reject empty input at
runtime and do not define `maxItems` in the initial design.

Repeat is accepted for SELECT queries, mutations, and fragments used by
Slot/Fragment composition. Repeat may contain Params, but it may not contain Slots
or nested Repeat ranges.

## Near-Term Direction

The near-term direction is to stabilize the current SELECT builder workflow while
implementing ADR 0010 and then ADR 0011 in focused slices:

- keep contributor and user documentation aligned with the supported and accepted
  post-MVP scope.
- keep diagnostics and CLI help explicit about supported dialects, statement
  kinds, target languages, and Param syntax.
- preserve generated-code driver independence while documenting practical
  `mysql2/promise` execution patterns for mutation builders.
- maintain examples, fixtures, and generated TypeScript artifacts as executable
  coverage for the supported workflow.

Larger expansions should be captured in ADRs before implementation.

Configuration placement matters for generated path preservation. If SQL files live
in a sibling directory such as `sql/` next to `configs/`, place
`sqlay.config.json` at the common project root instead of including `../sql/**`
from a nested config file.

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
- [ADR 0010: Define Initial MySQL Mutation Builder Support](./adr/0010-define-initial-mysql-mutation-builder-support.md)
- [ADR 0011: Define Repeat for Variable-Length SQL Repetition](./adr/0011-define-repeat-for-variable-length-sql-repetition.md)

## Out Of Scope

The following remain intentionally unsupported:

- `Slot` and `Fragment` features outside the initial ADR 0009 and ADR 0010 design,
  including required slots, default fragments, multi-select slots, fragment-local
  slots, fragment include or alias, and result-shape-changing SELECT variants.
- optional direct Param input properties that would require SQL structure changes.
- `Repeat` behavior outside ADR 0011, including empty-array fallback SQL,
  `maxItems` or `minItems`, scalar array inputs, exported Repeat item type aliases,
  nested Repeat ranges, Slot markers inside Repeat ranges, and Param-less
  count-based SQL duplication.
- mutation forms outside ADR 0010, including multi-table `UPDATE` and `DELETE`,
  `INSERT ... SELECT`, `REPLACE ... SELECT`, top-level CTE mutations, `CALL`,
  `LOAD DATA`, `TRUNCATE`, DDL, transaction control, and administrative
  statements.
- multi-statement source units.
- generated database execution functions.
- non-MySQL dialects.
- non-TypeScript target generators.
- automatic naming transformation.
- implicit `.env` loading.
