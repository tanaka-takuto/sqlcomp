# SQL Fixtures

These fixtures support SQL source and MySQL 8.x metadata integration work for the
MVP and post-MVP Param, Slot/Fragment, and mutation coverage. They are test
fixtures, not user-facing examples.

- `schema.sql` resets the metadata-oriented MySQL tables used by these fixtures,
  such as `fixture_all_column_type` and `fixture_child`.
- `seed.sql` inserts deterministic rows for metadata checks.
- `sqlay.valid.config.json` compiles `valid/**/*.sql`.
- `sqlay.invalid.config.json` points at `invalid/**/*.sql` for source-level
  diagnostics fixtures.
- `valid/type_metadata_matrix.sql` contains `@sqlay` query blocks that exercise
  result metadata for direct columns, aliases, joins, expressions, aggregate
  expressions, nullable columns, non-null columns, non-identifier column names, and
  MySQL type coverage.
- `valid/generation_surface.sql` exercises generated TypeScript surface behavior
  such as escaped SQL literals, inferred `LIMIT 1` cardinality, and explicit
  cardinality overrides.
- `valid/param_bindings.sql` exercises Param binding behavior, including direct
  column inference, inline `valueType`, `nullable: true`, repeated Param IDs, and
  `IN` predicates with one placeholder per marker.
- `valid/slot_fragment_composition.sql` and
  `valid/slot_fragment_fragments.sql` exercise valid Slot/Fragment composition,
  including cross-file fragments, fragment-only source files, mixed query/fragment
  source files, repeated Slot IDs, stable variants, and generated runtime branch
  builders.
- `valid/mutation_builders.sql` exercises valid mutation builders, including
  `INSERT`, `UPDATE`, `DELETE`, `REPLACE`, direct Param inference, explicit
  `valueType`, and mutation Slot/Fragment composition.
- `valid/nested/path_mapping.sql` verifies that generated output preserves nested
  config-relative source paths.
- `generated/` contains committed generated TypeScript expected artifacts.
- `invalid/` contains negative SQL source, Param, Slot/Fragment, and mutation
  diagnostics fixtures.

From the repository root, run the DB-backed fixture check against a running MySQL
service:

```sh
DATABASE_URL='mysql://sqlay:sqlay@127.0.0.1:3306/sqlay' script/check-mysql-fixtures.sh
```
