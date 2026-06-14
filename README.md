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

The local MySQL service loads fixture DDL and seed data from
`fixtures/mysql/init/` when the database volume is first created. Reset the fixture
state with:

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
script/check-all.sh
```

Type-check the generated TypeScript fixture directly with:

```sh
npm run typecheck:generated
```

Run the MySQL-backed integration checks against a running MySQL service with:

```sh
DATABASE_URL='mysql://sqlcomp:sqlcomp@127.0.0.1:3306/sqlcomp' script/mysql-integration.sh
```
