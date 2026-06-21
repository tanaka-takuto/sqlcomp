# ADR 0001: Use MySQL 8.x as the MVP Dialect

## Status

Accepted

## Context

`sqlay` is intended to support multiple SQL dialects over time. The MVP still needs
one concrete dialect so parsing, metadata extraction, and generated type mapping can
be validated end to end.

## Decision

The MVP targets official MySQL 8.x only.

MariaDB, Vitess, Aurora-specific behavior, PostgreSQL, SQLite, and other dialects are
outside the MVP.

## Consequences

- Query validation and result metadata extraction should use MySQL 8.x semantics.
- Generated TypeScript type mapping should be based on MySQL column metadata.
- Compatibility with MariaDB or MySQL-compatible distributed systems must not be
  assumed without a later ADR.
- The architecture should still keep dialect boundaries visible so future dialects
  can be added intentionally.
