#!/bin/sh

set -eu

if [ -n "${HOME:-}" ] && [ -f "$HOME/.cargo/env" ]; then
  . "$HOME/.cargo/env"
fi

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH= cd "$script_dir/.." && pwd)

require_command() {
  command_name=$1
  install_hint=$2

  if command -v "$command_name" >/dev/null 2>&1; then
    return 0
  fi

  cat >&2 <<EOF
check-examples: $command_name is required.

Install:
  $install_hint
EOF
  exit 1
}

select_mysql_client() {
  if command -v mysql >/dev/null 2>&1; then
    mysql_client=host
    return 0
  fi

  mysql_client=compose
  "$script_dir/dev/compose.sh" up
}

parse_database_url() {
  if [ -z "${DATABASE_URL:-}" ]; then
    cat >&2 <<'EOF'
check-examples: DATABASE_URL is required.

Example:
  DATABASE_URL='mysql://sqlcomp:sqlcomp@127.0.0.1:3306/sqlcomp' script/check-examples.sh
EOF
    exit 1
  fi

  case "$DATABASE_URL" in
    mysql://*@*/*) ;;
    *)
      cat >&2 <<'EOF'
check-examples: DATABASE_URL must use the form mysql://user:password@host:port/database.
EOF
      exit 1
      ;;
  esac

  url_without_scheme=${DATABASE_URL#mysql://}
  credentials=${url_without_scheme%%@*}
  location_and_database=${url_without_scheme#*@}
  host_port=${location_and_database%%/*}
  database_name=${location_and_database#*/}
  database_name=${database_name%%\?*}
  database_user=${credentials%%:*}
  database_password=${credentials#*:}

  if [ "$database_user" = "$credentials" ] || [ -z "$database_user" ] || [ -z "$database_password" ]; then
    cat >&2 <<'EOF'
check-examples: DATABASE_URL must include both user and password.
EOF
    exit 1
  fi

  database_host=${host_port%%:*}
  if [ "$database_host" = "$host_port" ]; then
    database_port=3306
  else
    database_port=${host_port#*:}
  fi

  if [ -z "$database_host" ] || [ -z "$database_port" ] || [ -z "$database_name" ]; then
    cat >&2 <<'EOF'
check-examples: DATABASE_URL must include host, port, and database.
EOF
    exit 1
  fi
}

load_mysql_file() {
  file=$1

  case "$mysql_client" in
    host)
      MYSQL_PWD=$database_password mysql \
        --protocol=TCP \
        -h "$database_host" \
        -P "$database_port" \
        -u "$database_user" \
        --database="$database_name" \
        < "$file"
      ;;
    compose)
      "$script_dir/dev/compose.sh" exec -T mysql \
        env MYSQL_PWD="$database_password" \
        mysql \
        --protocol=TCP \
        -h "$database_host" \
        -P "$database_port" \
        -u "$database_user" \
        --database="$database_name" \
        < "$file"
      ;;
    *)
      echo "check-examples: no MySQL client selected" >&2
      exit 1
      ;;
  esac
}

compare_directories() {
  expected_dir=$1
  actual_dir=$2
  expected_list=$tmp_root/expected-files.txt
  actual_list=$tmp_root/actual-files.txt

  if [ ! -d "$expected_dir" ]; then
    echo "check-examples: expected directory does not exist: $expected_dir" >&2
    exit 1
  fi

  if [ ! -d "$actual_dir" ]; then
    echo "check-examples: actual directory does not exist: $actual_dir" >&2
    exit 1
  fi

  (cd "$expected_dir" && find . -type f | LC_ALL=C sort) > "$expected_list"
  (cd "$actual_dir" && find . -type f | LC_ALL=C sort) > "$actual_list"

  if ! diff -u "$expected_list" "$actual_list"; then
    echo "check-examples: generated file list differs" >&2
    exit 1
  fi

  while IFS= read -r relative_path; do
    if ! cmp -s "$expected_dir/$relative_path" "$actual_dir/$relative_path"; then
      echo "check-examples: generated file differs: ${relative_path#./}" >&2
      diff -u "$expected_dir/$relative_path" "$actual_dir/$relative_path" || true
      exit 1
    fi
  done < "$expected_list"
}

require_command "cargo" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
require_command "npm" "install Node.js from https://nodejs.org/"

parse_database_url
select_mysql_client

tmp_root=$(mktemp -d "${TMPDIR:-/tmp}/sqlcomp-examples.XXXXXX")
trap 'rm -rf "$tmp_root"' EXIT HUP INT TERM

example_root=$repo_root/examples/bookstore
tmp_example=$tmp_root/bookstore

cp -R "$example_root" "$tmp_example"
rm -rf "$tmp_example/generated"

load_mysql_file "$example_root/schema.sql"
load_mysql_file "$example_root/seed.sql"

cd "$repo_root"
DATABASE_URL=$DATABASE_URL cargo run --locked -- compile --config "$tmp_example/sqlcomp.config.json"
compare_directories "$example_root/generated" "$tmp_example/generated"
npm exec -- tsc --noEmit --project "$tmp_example/tsconfig.json"
