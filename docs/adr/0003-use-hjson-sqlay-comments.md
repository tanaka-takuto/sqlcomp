# ADR 0003: Use Hjson `@sqlay` Comments

## Status

Accepted

## Context

`sqlay` needs metadata that can live inside SQL files without preventing those
files from being read as SQL. The metadata also needs room to grow from `Query` to
future `Param`, `Slot`, and `Fragment` concepts.

## Decision

Use SQL block comments with an `@sqlay` marker and an Hjson payload.

The canonical form is:

```sql
/* @sqlay
{
  type: query
  id: listUsers
}
*/
SELECT id, name FROM users;
```

## Consequences

- SQL tools should treat `@sqlay` metadata as comments.
- Metadata can remain readable as attributes grow.
- The first implementation should validate that the chosen Rust Hjson parser is
  reliable enough for diagnostics and typed deserialization.
- If Hjson parser support is not practical, the project should record a later ADR
  before narrowing the accepted syntax.
