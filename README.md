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
DATABASE_URL='mysql://user:password@localhost:3306/app' sqlcomp check
DATABASE_URL='mysql://user:password@localhost:3306/app' sqlcomp compile
```

`sqlcomp.config.json` is the project configuration file. It is parsed as JSON with
comments and trailing commas allowed. The MVP CLI does not implicitly load `.env`
files.
