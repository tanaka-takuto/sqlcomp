# ADR 0010: Define Initial MySQL Mutation Builder Support

## Status

Accepted

## Context

`sqlay` is SQL-first and currently keeps generated TypeScript independent from any
database driver. The existing SELECT builder workflow gives typed SQL text,
parameter arrays, and result row types, but applications that create, update, or
delete data still need a first-class way to generate typed DML builders.

Mutation support must improve adoption for normal application write paths without
turning generated code into an ORM or a database execution layer. MySQL write
statements also have execution-result semantics that are driver and statement
dependent. For example, `insertId` and affected row counts are available after
execution, but multi-row inserts, upserts, and `REPLACE` are not a portable
"return inserted rows" mechanism.

## Decision

### Source Units

Add a global `type: mutation` source unit:

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

Initial mutation metadata accepts only `type` and `id`. Statement kind is derived
from SQL, not duplicated in metadata. `query`, `mutation`, and `fragment` IDs share
one global namespace across the full compile run.

Source intake should preserve source order with a source-unit representation such
as `RawSourceUnit::Query`, `RawSourceUnit::Mutation`, and
`RawSourceUnit::Fragment`. Query and mutation source units generate builders;
fragment source units are resolved in the context of the query or mutation slots
that reference them.

### Statement Scope

Initial mutation support accepts these MySQL statement families:

- `INSERT`.
- `UPDATE`.
- `DELETE`.
- `REPLACE`.

The initial accepted forms are:

- `INSERT ... VALUES`.
- `INSERT ... SET`.
- `INSERT ... ON DUPLICATE KEY UPDATE`.
- `REPLACE ... VALUES`.
- `REPLACE ... SET`.
- single-table `UPDATE`, including table alias, `ORDER BY`, and `LIMIT`.
- single-table `DELETE`, including table alias, `ORDER BY`, and `LIMIT`.

`UPDATE` and `DELETE` require a `WHERE` clause. Initial support only checks for
the presence of `WHERE`; it does not prove that the predicate is selective or safe.

The following remain out of scope for initial mutation support:

- multi-table `UPDATE` and `DELETE`.
- `INSERT ... SELECT` and `REPLACE ... SELECT`.
- top-level CTE mutation forms such as `WITH ... UPDATE`.
- `CALL`, `LOAD DATA`, `TRUNCATE`, DDL, transaction control, and administrative
  statements.
- multi-statement source units.

Subqueries inside otherwise supported mutation statements are allowed. Param type
inference inside subqueries is limited to the already supported SELECT direct column
contexts, or otherwise requires `valueType`.

### Param Binding and Type Inference

Raw MySQL `?` placeholders remain unsupported in source SQL. Mutation authors use
paired inline `Param` markers around sample SQL expressions so source SQL remains
readable and executable in database tools.

`check` and `compile` must never execute mutation SQL. Database access for mutation
builders is limited to schema metadata needed for Param type inference.

Initial mutation Param inference is limited to direct column contexts:

- `INSERT` and `REPLACE` column lists mapped to `VALUES` positions.
- `INSERT ... SET column = param`.
- `REPLACE ... SET column = param`.
- `UPDATE ... SET column = param`.
- `INSERT ... ON DUPLICATE KEY UPDATE column = param`.
- qualified column comparisons such as `alias.column = param` or
  `param = alias.column`.
- supported `IN` predicates where each Param marker maps to one placeholder.

Column-list-free `INSERT INTO table VALUES (...)` and `REPLACE INTO table
VALUES (...)` are allowed as SQL, but their Params require `valueType` because
ordinal-position inference is too fragile.

Unqualified predicate columns do not drive inference, even in a single-table
mutation. `WHERE id = param` therefore requires `valueType`; `WHERE u.id = param`
can infer from alias `u`.

Only direct assignment forms infer from the target column. Params inside expressions
or functions require `valueType`, including examples such as
`SET count = count + param` or `LOWER(param)`.

Sample SQL literals are not used for type inference.

### Slot and Fragment Composition

Fragments remain global source units and are not marked as query-only or
mutation-only. A fragment is valid only in the insertion context where a query or
mutation slot uses it.

Mutation slots use the same initial Slot model as query slots:

- optional single-select.
- no required slots.
- no default fragments.
- no multi-select slots.
- no fragment-local slots.

The compiler does not add, remove, or normalize whitespace, commas, column lists,
or SQL keywords around mutation slots. Every Slot expansion variant must parse and
validate as a supported single mutation statement. For `UPDATE` and `DELETE`, every
variant must still have a `WHERE` clause. Every variant must have the same mutation
statement kind as the all-slots-unselected base variant.

Mutation variant validation does not compare result row shape or result
cardinality. It validates statement kind stability, supported statement form,
single-statement shape, Param type consistency, and mutation-specific safety rules.

Direct Param IDs and Slot IDs share the generated top-level input namespace and
must not collide. Fragment Params remain nested inside the selected slot branch.

### Generated TypeScript

Generated mutation builders return SQL text and params only. They do not execute
statements and do not generate driver-specific result types.

For `id: createUser`, generated symbols are:

- `createUser_Input`.
- `createUser`.

Mutation builders do not generate `createUser_Row` or `createUser_Output`.

Slotless mutation builders return fixed readonly params tuples. Slot mutation
builders return `readonly SqlParam[]`, matching the existing Slot query behavior
where branch selection can change the runtime parameter shape.

One SQL file may contain queries, mutations, and fragments. The generated
TypeScript file preserves source order: a query emits `Input`, `Row`, `Output`, and
function declarations; a mutation emits `Input` and function declarations.

### CLI and Summaries

`check` and `compile` should describe the aggregate generated surface as builders,
with query and mutation counts as separate details. For example:

```text
Compiled 5 builders: 3 queries, 2 mutations.
```

Warnings for included SQL files that contain SQL text but no top-level source unit
should mention both `type: query` and `type: mutation` annotations.

### Driver Guidance

Generated TypeScript stays driver-independent. User-facing docs and examples must
show how to execute mutation builders with `mysql2/promise`, including:

- reading `insertId` for single-row inserts.
- reading affected row counts for write statements.
- using the same connection when executing inside a transaction.

Docs must not recommend deriving multi-row inserted IDs with `insertId + index`.
When callers need inserted rows after multi-row inserts, they should use
application-generated unique keys or natural keys and issue an explicit SELECT.

Docs must also avoid using `insertId` or affected row counts as the official way to
classify `INSERT ... ON DUPLICATE KEY UPDATE` as inserted versus updated. When the
caller needs the final row, it should re-select by a stable unique key.

`REPLACE` is supported as SQL, but docs must describe its MySQL semantics clearly:
it can delete an existing row and insert a new one, which affects foreign keys,
triggers, auto-increment values, and affected row counts.

### Core IR and Architecture

Keep SELECT query IR and mutation IR separate. `RawQuery` and `CompiledQuery` stay
SELECT-specific. Add mutation-specific IR such as `RawMutation`,
`AnalyzedMutation`, and `CompiledMutation`. Use an aggregate builder representation,
such as `CompiledBuilder`, where application flow or target generation needs to
preserve mixed source order.

## Consequences

The implementation should be split into reviewable stages:

1. Add this ADR and update the source-of-truth product and architecture docs.
2. Add source intake and Core IR for mutation source units.
3. Add MySQL mutation analysis and schema-only Param inference.
4. Add application compile flow and mutation Slot variant validation.
5. Add TypeScript mutation builder generation.
6. Add valid and invalid fixtures.
7. Add user-facing examples and docs, including `mysql2/promise` execution
   patterns.

Initial mutation support is a typed SQL builder feature, not an ORM replacement.
Execution, result objects, transaction helpers, and database-driver abstractions
remain outside generated code.

## Future Work

Later ADRs may define:

- `Repeat` for variable-length SQL repetition such as bulk `VALUES` rows or
  dynamic `IN` lists.
- multi-table `UPDATE` and `DELETE`.
- `INSERT ... SELECT` and `REPLACE ... SELECT`.
- top-level CTE mutation forms.
- required slots, default fragments, and multi-select slots.
- explicit full-table mutation opt-in for `UPDATE` or `DELETE` without `WHERE`.
- driver-specific execution helpers.

## Alternatives Considered

Use `type: query` plus `cardinality: exec` for DML. This was rejected because
SELECT queries and DML mutations have different generated surfaces and metadata
requirements.

Generate database execution functions. This was rejected because it would add
driver choice, transaction behavior, execution result typing, and connection
management to a feature whose first responsibility is typed SQL construction.

Infer Param types from sample literals. This remains rejected because SQL literals
are ambiguous without database context and would weaken the explicit type contract.

Use `insertId + index` as the documented multi-row insert result pattern. This was
rejected because MySQL execution semantics do not make that a robust general way to
map input rows to generated IDs.
