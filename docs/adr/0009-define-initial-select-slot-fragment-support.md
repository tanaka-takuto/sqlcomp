# ADR 0009: Define Initial SELECT Slot/Fragment Support

## Status

Accepted

## Context

`sqlay` currently supports MySQL `SELECT` query builders with inline `Param`
value binding. The next dynamic SQL capability is `Slot` and `Fragment`
composition, where a query can select from named SQL fragments while still
preserving the 2-way SQL philosophy from ADR 0003 and explicit naming rules from
ADR 0005.

Slot/Fragment support crosses source intake, global resolution, SQL expansion,
MySQL-backed validation, Core IR, TypeScript generation, examples, fixtures, docs,
and CLI summaries. Without one design, those implementation slices could choose
incompatible source shapes or generated APIs.

## Decision

Initial Slot/Fragment support is limited to MySQL `SELECT` query builders and
TypeScript SQL builder generation. It composes SQL text before validation and does
not add generated database execution functions.

### Source Units

`fragment` is a global source unit like `query`. Query IDs and fragment IDs share
one global namespace across the full compile run. All user-defined IDs use the same
portable identifier rule as query IDs and Param IDs:

```text
^[A-Za-z_][A-Za-z0-9_]*$
```

Initial fragment metadata accepts only `type` and `id`:

```sql
/* @sqlay
{
  type: fragment
  id: activeOnly
}
*/
AND u.active = 1
```

A fragment body ends at the next global `query` or `fragment` annotation, or at the
end of the file. A fragment body does not require a trailing semicolon. A raw
statement separator `;` is rejected in fragment bodies outside SQL literals.

A fragment is valid only in insertion context. It is not a standalone SQL statement,
and the compiler does not validate a fragment by itself without a query slot that
uses it.

`slot` is a query-body inline marker and represents a zero-width insertion point.
Initial slot metadata accepts only `type`, `id`, and `targets`:

```sql
SELECT u.id, u.email
FROM users AS u
WHERE 1 = 1
/* @sqlay { type: slot id: filter targets: [activeOnly, byEmail] } */;
```

`slot.targets` is a non-empty string array of global fragment IDs. Initial slots are
optional single-select slots: each slot input may select exactly one target fragment
or select no fragment. Required slots and default fragments are not part of the
initial design.

For expansion, the slot marker comment is replaced by either the selected fragment
SQL or the empty string. The compiler does not add, trim, or normalize whitespace
around the insertion point. Source authors are responsible for writing query and
fragment whitespace that composes into valid SQL.

`param` markers are allowed in query bodies and fragment bodies. `slot` markers
inside fragments are rejected for the initial release, but the syntax is reserved
for a later composition step. Fragment include, alias, and extends concepts are not
introduced.

Generated SQL contains no `@sqlay` comments. Ordinary SQL comments are preserved.

### Resolution and Input Shape

Fragment targets are resolved globally. A slot target must name an existing fragment
ID in the shared query/fragment namespace.

Query direct Params and Slots share the generated query input top-level namespace
and may not collide. Fragment Params are nested under the selected slot branch input
instead of being lifted to the query top level.

Slot branch input uses `$fragment` as the discriminant. The top-level slot property
is optional; an absent property means the optional slot is unselected. A branch for a
fragment includes that fragment's Param inputs:

```ts
export type listUsers_Input = {
  filter?: { $fragment: "activeOnly" } | {
    $fragment: "byEmail";
    email: string;
  };
};
```

Fragment input types are not exported independently. They appear only as part of the
query input types that use them.

Repeated `slot.id` occurrences are allowed within one query when their `targets`
arrays match exactly, including order. Repeated occurrences share the same generated
selection input.

### Expansion and Validation

The compiler enumerates expansion variants per unique slot ID, not per slot marker
occurrence. Each unique slot contributes one unselected choice plus one choice for
each target. Variant enumeration follows unique slot first-seen order. Within a
slot, the unselected choice comes first, then targets appear in source order.

The initial variant limit is 256. A query whose unique slots produce more than 256
variants is rejected.

All variants are validated during `check` and `compile`. Expanded SQL variants are
not deduplicated initially, even if two choices produce identical SQL.

Every variant must have stable cardinality and result row shape. Row shape
comparison uses column order, column name, CoreType, and nullability. An explicit
query `cardinality` override applies before cardinality comparison. After successful
validation, base variant metadata is used for generated row and output types.

Fragment Param types are inferred in the query context of each slot expansion. If
the same repeated slot ID can select the same fragment at multiple insertion points,
that fragment's Param types and nullability must be consistent across occurrences.

### Generated TypeScript

Slotless query generation remains unchanged.

Slot queries generate runtime SQL branches using `sqlParts.push(...)` and
`sqlParts.join("")`. Generated builders do not perform runtime validation. Generated
switch statements do not include a default case for unknown `$fragment` values.

Generated builders append `params` values in the exact order their corresponding
`?` placeholders appear in the expanded SQL. Direct query Params and selected
fragment Params are therefore interleaved according to SQL emission order, not
grouped by source unit. When one repeated slot input is inserted at multiple slot
occurrences, the same input value is appended at each selected fragment placeholder
occurrence in expanded-SQL placeholder order. Unselected slots append no params.

Because selected branches can change the Param tuple shape at runtime, slot queries
return `params: readonly SqlParam[]`. A generated TypeScript file that contains at
least one slot query emits one private file-level helper alias, shared by all slot
queries in that file:

```ts
type SqlParam = unknown;
```

### Files and Summaries

Fragment-only SQL files do not generate TypeScript output files. Cross-file
fragments are embedded into the generated query file that uses them.

CLI summaries include query, fragment, slot, and variant counts. Per-query summaries
include Param, slot, and variant counts.

## Consequences

The feature can be implemented in small slices:

- Source intake parses global `fragment` units and query-local `slot` markers.
- Inline annotation placement rules are shared by `Param` and `Slot`.
- Source intake preserves SQL text needed for replacement and diagnostics.
- Global resolution validates shared query/fragment IDs and slot targets.
- Variant expansion builds each SQL string that MySQL validation must check.
- MySQL validation, row shape validation, and cardinality validation run for every
  variant.
- Core IR represents dynamic slot selection without changing slotless query IR.
- TypeScript generation emits `$fragment` discriminated slot inputs and runtime SQL
  branches.
- Output planning skips fragment-only files while embedding cross-file fragments in
  using query files.
- Fixtures, examples, README content, and CLI summaries cover the supported shape.

This ADR approves the initial design, but support is only available as the
implementation issues land.

## Out of Scope

The initial Slot/Fragment release does not include:

- required slots.
- default fragments.
- multi-select slots.
- fragment-local slots.
- fragment include or alias.
- result-shape-changing Slot/Fragment support.
- row union output for variant-dependent columns.
- independently exported fragment TypeScript input types.

## Alternatives Considered

Inline fragment definitions scoped to one query were rejected because cross-query
reuse is the primary reason to introduce fragments.

Fragment-side `targets` were rejected because the query slot should own the allowed
choice set at each insertion point.

Required slots and default fragments were deferred to keep the first release focused
on optional single-select composition.

Exporting fragment input types independently was rejected because the same fragment
can have query-context-dependent Param inference.

Runtime validation for unknown `$fragment` values was rejected to keep generated
builders small and to rely on TypeScript for caller-side correctness.

Row union output for result-shape-changing fragments was rejected because the initial
release requires stable result metadata across all variants.

## See Also

- [ADR 0002: Use TypeScript SQL Builders as the First Target Generator](./0002-use-typescript-target-generator-first.md)
- [ADR 0003: Use Hjson `@sqlay` Comments](./0003-use-hjson-sqlay-comments.md)
- [ADR 0005: Do Not Automatically Transform Generated Names](./0005-do-not-transform-generated-names.md)
- [ADR 0008: Define SELECT Param Support](./0008-define-select-param-support.md)
