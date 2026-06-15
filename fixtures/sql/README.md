# SQL Fixtures

These fixtures support SQL source and MySQL 8.x metadata integration work for the
MVP. They are test fixtures, not user-facing examples.

- `schema.sql` resets the metadata-oriented MySQL tables used by these fixtures,
  such as `fixture_all_column_type` and `fixture_child`.
- `seed.sql` inserts deterministic rows for metadata checks.
- `sqlcomp.valid.config.json` compiles `valid/**/*.sql`.
- `sqlcomp.invalid.config.json` points at `invalid/**/*.sql` for source-level
  diagnostics fixtures.
- `valid/type_metadata_matrix.sql` contains `@sqlcomp` query blocks that exercise
  result metadata for direct columns, aliases, joins, expressions, aggregate
  expressions, nullable columns, non-null columns, non-identifier column names, and
  MySQL type coverage.
- `valid/generation_surface.sql` exercises generated TypeScript surface behavior
  such as escaped SQL literals, inferred `LIMIT 1` cardinality, and explicit
  cardinality overrides.
- `valid/nested/path_mapping.sql` verifies that generated output preserves nested
  config-relative source paths.
- `generated/` contains committed generated TypeScript expected artifacts.
- `invalid/` contains negative SQL source fixtures.

From the repository root, run the DB-backed fixture check against a running MySQL
service:

```sh
DATABASE_URL='mysql://sqlcomp:sqlcomp@127.0.0.1:3306/sqlcomp' script/check-mysql-fixtures.sh
```
