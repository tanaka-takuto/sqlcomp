# ADR 0006: Define MVP CLI, Config, and Generation Workflow

## Status

Accepted

## Context

The MVP needs a stable user-facing workflow before implementation begins. The
configuration format, command surface, database connection behavior, and generated
file rules are difficult to change after users start wiring the tool into projects
and CI.

The project should stay approachable for TypeScript and SQL users rather than
exposing Rust-specific conventions as the primary interface.

## Decision

The MVP uses `sqlay.config.json` as the standard configuration file name. The
file is parsed as JSON with comments and trailing commas allowed. This keeps the
file familiar to TypeScript ecosystem users while still allowing commented local
configuration.

The MVP configuration shape is nested by responsibility:

```jsonc
{
  "source": {
    "include": ["sql/**/*.sql"],
    "exclude": [],
  },
  "output": {
    "dir": "src/generated/sqlay",
  },
  "database": {
    "dialect": "mysql",
    "urlEnv": "DATABASE_URL",
  },
  "target": {
    "language": "typescript",
  },
}
```

For the MVP, `source.include`, `output.dir`, `database.dialect`,
`database.urlEnv`, and `target.language` are required. `source.exclude` is
optional.

When `--config` is not provided, `sqlay` searches from the current working
directory upward for `sqlay.config.json`. Paths inside the configuration are
resolved relative to the directory containing that configuration file. Generated
TypeScript files preserve the input SQL path relative to that same configuration
directory. Matched source files must stay inside the configuration directory;
projects with SQL files in sibling directories should place `sqlay.config.json`
at their common project root.

The MVP exposes three commands:

- `sqlay init` creates a config template and refuses to overwrite an existing
  config file.
- `sqlay check` runs the full compile pipeline, including MySQL metadata
  lookup, but does not write generated files.
- `sqlay compile` writes generated TypeScript files.

Database connection URLs are read from the process environment using the
configured `database.urlEnv` name. The CLI does not load `.env` files implicitly.

Generated files include a generated-code header. `sqlay compile` treats
`output.dir` as a generated area and overwrites same-path files. Stale generated
files are removed only when `sqlay compile --clean` is used.

The project should provide a local MySQL 8.x development environment and run
MySQL-backed integration checks in CI before the MVP is considered complete.

## Consequences

- The MVP has a small but complete command surface for project setup, CI checks,
  and code generation.
- TypeScript users see a familiar JSON-style configuration file rather than TOML.
- Output path behavior is deterministic because the config file location is the
  path base.
- Nested alternate configs cannot include `../sql/**` from outside their
  configuration directory without a later design change to the path-safety and
  output-relative-path model.
- Users must keep `output.dir` dedicated to generated files if they rely on the
  default overwrite behavior.
- Local `.env` loading can be added later only by explicit design decision.
