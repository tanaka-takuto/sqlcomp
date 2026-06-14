#!/bin/sh

set -eu

if [ -n "${HOME:-}" ] && [ -f "$HOME/.cargo/env" ]; then
  . "$HOME/.cargo/env"
fi

if ! command -v cargo >/dev/null 2>&1; then
  cat >&2 <<'EOF'
mysql-integration: cargo is required.

Install:
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
EOF
  exit 1
fi

if [ -z "${DATABASE_URL:-}" ]; then
  cat >&2 <<'EOF'
mysql-integration: DATABASE_URL is required.

Example:
  DATABASE_URL='mysql://sqlcomp:sqlcomp@127.0.0.1:3306/sqlcomp' script/mysql-integration.sh
EOF
  exit 1
fi

cargo test --locked -p sqlcomp-adapters --all-features --tests -- --ignored --nocapture
