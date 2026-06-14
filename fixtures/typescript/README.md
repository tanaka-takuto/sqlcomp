# TypeScript Fixture

This fixture type-checks representative generated SQL builder files without a
database driver dependency.

Run it from the repository root:

```sh
npm ci
npm run typecheck:generated
```

The check intentionally uses `tsc --noEmit` so it validates TypeScript syntax and
public type surfaces without producing JavaScript output.
