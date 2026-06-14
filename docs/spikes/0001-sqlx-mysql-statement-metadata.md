# Spike 0001: sqlx MySQL Statement Metadata

## Summary

`sqlx` can expose the MySQL statement metadata required by the MVP for prepared
`SELECT` queries: result column names, database type names, and nullability.

The MVP should continue with `sqlx` as the MySQL metadata provider. A client-choice
ADR is not needed from this spike.

## Scope

This spike validates metadata lookup only. It does not implement the production
`MetadataProvider` adapter.

The executable check is:

```sh
docker compose up -d --wait mysql
DATABASE_URL='mysql://sqlcomp:sqlcomp@127.0.0.1:3306/sqlcomp' \
  cargo test --locked -p sqlcomp-adapters --test mysql_metadata_spike -- --ignored --nocapture
```

The test creates a fixture table and calls `describe` for representative `SELECT`
queries. It does not insert rows or fetch query result rows.

## sqlx APIs

The spike uses `sqlx` 0.9.0 with these dev-dependency features:

```toml
sqlx = { version = "0.9.0", default-features = false, features = [
  "runtime-tokio",
  "mysql",
  "tls-rustls",
  "macros",
] }
```

Exact APIs used:

- `sqlx::MySqlConnection::connect(&database_url).await`
- `sqlx::raw_sql(...).execute(&mut connection).await` for fixture DDL
- `sqlx::Executor::describe(sql.into_sql_str()).await`
- `sqlx::Describe::columns()`
- `sqlx::Describe::nullable(index) -> Option<bool>`
- `sqlx::Column::name()`
- `sqlx::Column::type_info()`
- `sqlx::TypeInfo::name()`
- `sqlx::SqlSafeStr::into_sql_str()`

Important caveat: in `sqlx` 0.9.0, `Executor::describe` is `#[doc(hidden)]` and is
available through the `offline` cfg enabled by the `macros` feature. The ordinary
prepared statement API exposes columns and parameter counts, but not nullable
metadata.

## Results

Fixture table:

```sql
CREATE TABLE sqlcomp_metadata_spike_users (
  id BIGINT NOT NULL PRIMARY KEY,
  display_name VARCHAR(255) NOT NULL,
  nickname VARCHAR(255) NULL,
  age INT NULL,
  created_at DATETIME NOT NULL,
  deleted_at DATETIME NULL,
  balance DECIMAL(10, 2) NOT NULL,
  active TINYINT(1) NOT NULL,
  payload JSON NULL
);
```

Aliases and direct table columns:

| Column        | Type       | Nullable      |
| ------------- | ---------- | ------------- |
| `userId`      | `BIGINT`   | `Some(false)` |
| `displayName` | `VARCHAR`  | `Some(false)` |
| `nickname`    | `VARCHAR`  | `Some(true)`  |
| `createdAt`   | `DATETIME` | `Some(false)` |
| `deletedAt`   | `DATETIME` | `Some(true)`  |

Expressions:

| Column               | Type      | Nullable      |
| -------------------- | --------- | ------------- |
| `nextId`             | `BIGINT`  | `Some(false)` |
| `label`              | `VARCHAR` | `Some(true)`  |
| `nextAge`            | `BIGINT`  | `Some(true)`  |
| `isActiveExpression` | `BIGINT`  | `Some(false)` |

Mixed database types:

| Column    | Type      | Nullable      |
| --------- | --------- | ------------- |
| `balance` | `DECIMAL` | `Some(false)` |
| `active`  | `BOOLEAN` | `Some(false)` |
| `payload` | `JSON`    | `Some(true)`  |

Notable observations:

- Aliases are returned as the result column names. This matches the MVP rule that
  generated names should come from explicit SQL output names.
- MySQL nullable and non-null table columns return `Some(true)` and `Some(false)`.
- Expressions also return database type names and nullable metadata.
- `TINYINT(1)` is surfaced by `sqlx::MySqlTypeInfo::name()` as `BOOLEAN`.
- `deleted_at IS NULL` is surfaced as `BIGINT`, not `BOOLEAN`.

## Recommendation

Continue with `sqlx` for MVP MySQL metadata lookup.

The production adapter should:

- call `Executor::describe` after dialect analysis accepts exactly one `SELECT`
  statement.
- read output names from `Column::name()`.
- read database-native type names from `Column::type_info().name()`.
- read nullability from `Describe::nullable(index)`.
- map `None` nullability to nullable output in Core IR, as already required by the
  MVP.
- decide MySQL-to-Core type mapping explicitly, including `BOOLEAN` from
  `TINYINT(1)` and expression booleans reported as `BIGINT`.

Because `Executor::describe` is doc-hidden in `sqlx` 0.9.0, the adapter should keep
the dependency localized to `crates/adapters` and the project should revisit this
choice if a later `sqlx` release removes or materially changes this API.
