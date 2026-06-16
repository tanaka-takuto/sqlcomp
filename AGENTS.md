# AGENTS.md

## Source of Truth

Treat `docs/` as the authoritative source for product, architecture, current scope,
and historical MVP decisions.

Before making design-sensitive changes, read:

- `docs/current-scope.md`
- `docs/vision.md`
- `docs/architecture.md`
- `docs/mvp.md` for the completed initial MVP baseline
- relevant files in `docs/adr/`

Do not duplicate detailed design rules in this file. If a product or architecture
decision changes, update the relevant document or add a new ADR.

## Current Scope Summary

The current supported scope is query-only, SELECT-only, MySQL 8.x, and TypeScript
SQL builder generation, including SELECT value binding through inline `Param`
markers. `Slot`, `Fragment`, non-SELECT statements, optional input properties,
generated database execution functions, and additional dialects or target
generators remain out of scope unless a later ADR changes that scope.

## Development Notes

- Keep generated behavior explicit and predictable.
- Do not add automatic naming transformations without an ADR.
- Run the relevant checks for changed files.
- Markdown, JSON, and YAML files are formatted with dprint.
