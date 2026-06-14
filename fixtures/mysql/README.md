# MySQL Fixtures

These fixtures support MySQL 8.x metadata integration work for the MVP.

- `init/001_metadata_fixture.sql` is mounted into the local MySQL container and
  runs when a fresh database volume is created. It is intentionally shaped around
  direct result metadata coverage.
- `init/002_business_fixture.sql` adds a small practical commerce model for broader
  integration scenarios: customers, orders, and order items.
- `queries/metadata.sql` contains `@sqlcomp` query blocks that exercise result
  metadata for direct columns, aliases, joins, expressions, nullable columns, and
  non-null columns.
- `queries/business.sql` contains `@sqlcomp` query blocks that look like ordinary
  application reads while keeping the schema small: summaries, profile lookups,
  left joins, line totals, and case expressions.

From the repository root, reset the local database volume before reloading the init
fixture:

```sh
script/mysql-reset.sh
```
