# ADR 0005: Do Not Automatically Transform Generated Names

## Status

Accepted

## Context

The project philosophy favors explicit design and predictable generated code. Naming
transformations such as camelCase to PascalCase can look convenient but create hidden
rules, target-language assumptions, and edge cases.

## Decision

`sqlay` does not automatically transform query IDs into different case styles.

The query `id` must be a valid portable identifier:

```text
^[A-Za-z_][A-Za-z0-9_]*$
```

TypeScript generation uses the `id` exactly as written. Additional generated symbols
use fixed suffixes only:

- `<id>`
- `<id>_Input`
- `<id>_Row`
- `<id>_Output`

## Consequences

- Users are responsible for choosing IDs that are suitable for their target language.
- The compiler avoids implicit naming policy.
- TypeScript types may intentionally use names such as `listUsers_Row` instead of
  generated PascalCase names.
- Future target generators should either use the same explicit ID or require an
  explicit design decision before adding language-specific naming metadata.
