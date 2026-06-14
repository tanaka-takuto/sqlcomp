#!/bin/sh

set -eu

if ! command -v npm >/dev/null 2>&1; then
  cat >&2 <<'EOF'
typescript-generated-check: npm is required.

Install Node.js and npm, then run:
  npm ci
EOF
  exit 1
fi

if [ ! -x node_modules/.bin/tsc ]; then
  cat >&2 <<'EOF'
typescript-generated-check: TypeScript dependencies are not installed.

Install them from the pinned lockfile:
  npm ci
EOF
  exit 1
fi

npm run typecheck:generated
