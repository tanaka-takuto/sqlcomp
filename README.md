# sqlcomp

SQL Compose & Compile.

`sqlcomp` is a Rust CLI for writing plain SQL files while generating typed target
language builders. The current MVP focuses on MySQL 8.x `SELECT` queries and
TypeScript SQL builder generation.

See [`docs/`](./docs/) for the product, architecture, and MVP decisions.

## MVP Workflow

The planned MVP workflow is:

```sh
sqlcomp init
script/mysql-up.sh
DATABASE_URL='mysql://sqlcomp:sqlcomp@127.0.0.1:3306/sqlcomp' sqlcomp check
DATABASE_URL='mysql://sqlcomp:sqlcomp@127.0.0.1:3306/sqlcomp' sqlcomp compile
```

`sqlcomp.config.json` is the project configuration file. It is parsed as JSON with
comments and trailing commas allowed. The MVP CLI does not implicitly load `.env`
files.

## Local MySQL

The repository includes a Docker Compose service for local MySQL 8.x development:

```sh
script/mysql-up.sh
```

The service uses deterministic development-only credentials and waits for the
container healthcheck before the command exits.

Use this connection URL for local checks:

```sh
export DATABASE_URL='mysql://sqlcomp:sqlcomp@127.0.0.1:3306/sqlcomp'
```

The local MySQL service starts an empty development database. Example and fixture
checks load their own prefix-scoped schema and seed data each time they run. Reset
the database volume with:

```sh
script/mysql-reset.sh
```

Stop the service without removing the database volume with:

```sh
script/mysql-down.sh
```

## Local Checks

Run the same non-database baseline checks used by CI with:

```sh
npm ci
script/check-baseline.sh
```

Type-check committed generated TypeScript artifacts directly with:

```sh
npm run typecheck:examples
npm run typecheck:fixtures
```

Run the MySQL-backed example E2E check against a running MySQL service with:

```sh
DATABASE_URL='mysql://sqlcomp:sqlcomp@127.0.0.1:3306/sqlcomp' script/check-examples.sh
```

Run the MySQL-backed fixture checks with:

```sh
DATABASE_URL='mysql://sqlcomp:sqlcomp@127.0.0.1:3306/sqlcomp' script/check-mysql-fixtures.sh
```
