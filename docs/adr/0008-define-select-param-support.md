# ADR 0008: Define SELECT Param Support

## Status

Accepted

## Context

The MVP intentionally excludes `Param`, `Slot`, and `Fragment` support. ADR 0004
keeps the MVP limited to query-only MySQL `SELECT` support, while ADR 0002 already
reserves an input argument and `params` return field in generated TypeScript SQL
builders.

The first post-MVP `Param` feature needs a design that keeps SQL source valid as
2-way SQL, keeps `@sqlay` metadata in SQL comments as decided by ADR 0003, and
preserves the explicit naming rules from ADR 0005. Without a shared design, source
intake, MySQL analysis, database-backed type inference, Core IR, and TypeScript
generation could drift independently.

## Decision

Initial `Param` support is post-MVP and limited to value binding for `SELECT` query
builders. `Slot`, `Fragment`, non-`SELECT` statements, optional input properties, and
generated database execution functions remain out of scope.

SQL source uses paired inline block comments around a sample SQL expression:

```sql
SELECT id, email
FROM users
WHERE email = /* @sqlay { type: param id: email } */
  'test@example.test'
  /* @sqlay { type: paramEnd } */;
```

The SQL between `param` and `paramEnd` is a sample expression. It keeps the source
query directly executable in database tools and provides source text for diagnostics,
but it is not used to infer the parameter type.

Metadata values use camelCase for compound names. `paramEnd` is valid and
`param_end` is not. Unknown metadata fields are rejected.

Each `param` marker must provide an `id`. Param IDs use the same portable identifier
rule as query IDs:

```text
^[A-Za-z_][A-Za-z0-9_]*$
```

Query metadata does not gain a separate `params` list. Inline param markers are the
source of truth for param presence, order, names, nullability, and inline type
overrides.

`param` and `paramEnd` pairs are required. Ranges must not be nested. Each range must
cover a single SQL expression. A raw MySQL positional placeholder written directly as
`?` is rejected anywhere in a query block, and a sample expression containing `?` is
also rejected.

For dialect analysis, database metadata lookup, `check`, `compile`, and generated
TypeScript output, each param range is replaced by one MySQL positional placeholder
`?`. `IN` support binds one placeholder per marker and does not perform array
expansion.

Type inference is database-backed and initially limited to direct column context in
the current MySQL database:

- qualified real-table columns from the same `SELECT` statement's `FROM` and `JOIN`
  sources.
- comparison operators `=`, `<>`, `!=`, `<`, `<=`, `>`, and `>=`.
- `IN` predicates where each param marker maps to one placeholder.

Both `table_alias.column = param` and `param = table_alias.column` forms may use the
column context. Inference from unqualified columns, select-list aliases, derived
tables, subqueries, function return types, casts, or sample literal values is out of
scope. If inference is not possible, inline `valueType` is required.

`valueType` is a lower-case CoreType override such as `string`, `int64`, or
`datetime`. The accepted values are the lower-case names of non-unknown Core types.
`valueType` overrides database inference without a warning.

Inputs are non-null unless the marker explicitly sets `nullable: true`. Nullable
inputs are represented as `T | null` in generated TypeScript input and params types.
`nullable: false` is not part of the accepted metadata shape. Optional input
properties are not supported.

The same Param ID may appear multiple times in one query when all occurrences agree
on type and nullability. Generated params tuples follow marker occurrence order.
Generated input properties follow first Param ID occurrence order.

Generated TypeScript continues to use the existing CoreType-to-TypeScript mapping.
For a query with params, the generated builder input is required and the returned
`params` tuple is populated from input properties in marker occurrence order. Queries
without params keep the MVP empty input shape and empty readonly params tuple.

## Consequences

The feature can be split into focused implementation issues:

- Source intake parses inline `param` and `paramEnd` comments, validates marker
  pairing and marker metadata, and records source ranges for diagnostics.
- SQL replacement produces an analysis SQL string with `?` placeholders while
  preserving the original source text for diagnostics.
- The MySQL dialect analyzer validates the replaced SQL as one `SELECT` statement
  and rejects raw positional placeholders from source SQL.
- Database-backed inference resolves only the direct table-column contexts described
  above and requires `valueType` elsewhere.
- Core IR represents input fields and param occurrences separately so one input
  field can feed multiple params.
- The TypeScript generator emits input object types and readonly params tuples using
  existing CoreType mappings.
- Fixture and example coverage should include valid repeated IDs, direct comparison
  inference, `IN` binding, inline `valueType`, nullable inputs, raw `?` rejection,
  nested marker rejection, missing pair diagnostics, unknown metadata fields, and
  type/nullability conflicts.

This ADR does not change the MVP boundary. The MVP remains query-only, SELECT-only,
MySQL 8.x, and TypeScript SQL builder generation without dynamic composition.

See also:

- [ADR 0001: Use MySQL 8.x as the MVP Dialect](./0001-use-mysql-8-for-mvp.md)
- [ADR 0002: Use TypeScript SQL Builders as the First Target Generator](./0002-use-typescript-target-generator-first.md)
- [ADR 0003: Use Hjson `@sqlay` Comments](./0003-use-hjson-sqlay-comments.md)
- [ADR 0004: Limit the MVP to Query-only SELECT Support](./0004-limit-mvp-to-query-only-select.md)
- [ADR 0005: Do Not Automatically Transform Generated Names](./0005-do-not-transform-generated-names.md)

## Alternatives Considered

Use raw MySQL `?` placeholders in source SQL and map them to params by order. This is
simpler to compile, but it weakens 2-way SQL because the source query is not directly
executable with representative values.

Declare params only in query metadata. This separates public API declarations from
SQL usage sites, but duplicates information once type inference and inline overrides
are needed.

Infer param types from sample expressions. This is rejected because SQL literals are
ambiguous without database context and would introduce implicit behavior.

Support optional inputs immediately. This is rejected because optional parameters
imply SQL structure changes, which belongs to future `Slot` or `Fragment` support.
