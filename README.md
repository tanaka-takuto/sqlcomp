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
docker compose up -d --wait mysql
DATABASE_URL='mysql://sqlcomp:sqlcomp@127.0.0.1:3306/sqlcomp' sqlcomp check
DATABASE_URL='mysql://sqlcomp:sqlcomp@127.0.0.1:3306/sqlcomp' sqlcomp compile
```

`sqlcomp.config.json` is the project configuration file. It is parsed as JSON with
comments and trailing commas allowed. The MVP CLI does not implicitly load `.env`
files.

## Local MySQL

The repository includes a Docker Compose service for local MySQL 8.x development:

```sh
docker compose up -d --wait mysql
```

The service uses deterministic development-only credentials and waits for the
container healthcheck before the command exits.

Use this connection URL for local checks:

```sh
export DATABASE_URL='mysql://sqlcomp:sqlcomp@127.0.0.1:3306/sqlcomp'
```
