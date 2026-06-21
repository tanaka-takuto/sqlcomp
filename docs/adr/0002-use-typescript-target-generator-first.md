# ADR 0002: Use TypeScript SQL Builders as the First Target Generator

## Status

Accepted

## Context

`sqlay` is designed to generate code for statically typed languages. The MVP needs
one target generator that can prove query validation, result type extraction, and
generated API shape without taking on driver execution concerns.

## Decision

The first target generator emits TypeScript SQL builder code from Core IR.

Generated code returns SQL text and parameters. It does not execute queries and does
not depend on a TypeScript database driver.

## Consequences

- Generated TypeScript can be used with `mysql2`, Kysely, a custom database layer, or
  another caller-managed execution path.
- The MVP generated API includes input, row, and output types, even though MVP inputs
  are empty.
- Runtime dependencies in generated TypeScript should be avoided.
- Other language target generators remain future work.
