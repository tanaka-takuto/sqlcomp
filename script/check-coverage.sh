#!/bin/sh

set -eu

if [ -n "${HOME:-}" ] && [ -f "$HOME/.cargo/env" ]; then
  . "$HOME/.cargo/env"
fi

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH= cd "$script_dir/.." && pwd)
coverage_report=coverage/lcov.info
coverage_min_line_percent=85

require_command() {
  command_name=$1
  install_hint=$2

  if command -v "$command_name" >/dev/null 2>&1; then
    return 0
  fi

  cat >&2 <<EOF
check-coverage: $command_name is required.

Install:
  $install_hint
EOF
  exit 1
}

require_cargo_llvm_cov() {
  if cargo llvm-cov --version >/dev/null 2>&1; then
    return 0
  fi

  cat >&2 <<'EOF'
check-coverage: cargo-llvm-cov is required.

Install:
  cargo install cargo-llvm-cov --locked --version 0.8.7
EOF
  exit 1
}

require_command "cargo" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
require_cargo_llvm_cov

cd "$repo_root"
mkdir -p "$(dirname "$coverage_report")"
cargo llvm-cov \
  --workspace \
  --all-targets \
  --all-features \
  --fail-under-lines "$coverage_min_line_percent" \
  --lcov \
  --output-path "$coverage_report"
