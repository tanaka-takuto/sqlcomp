#!/bin/sh

set -eu

if [ -n "${HOME:-}" ] && [ -f "$HOME/.cargo/env" ]; then
  . "$HOME/.cargo/env"
fi

require_command() {
  command_name=$1
  install_hint=$2

  if command -v "$command_name" >/dev/null 2>&1; then
    return 0
  fi

  cat >&2 <<EOF
check-baseline: $command_name is required.

Install:
  $install_hint
EOF
  exit 1
}

require_command "dprint" "brew install dprint"
require_command "npm" "install Node.js from https://nodejs.org/"
require_command "cargo" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"

dprint check
npm run typecheck:examples
npm run typecheck:fixtures
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features
cargo test --locked --workspace --all-targets --all-features
