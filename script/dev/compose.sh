#!/bin/sh

set -eu

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH= cd "$script_dir/../.." && pwd)

usage() {
  cat >&2 <<'EOF'
Usage:
  script/dev/compose.sh up
  script/dev/compose.sh down
  script/dev/compose.sh reset
  script/dev/compose.sh ps
  script/dev/compose.sh logs [docker compose logs args...]
  script/dev/compose.sh exec [docker compose exec args...]
EOF
}

if ! command -v docker >/dev/null 2>&1; then
  cat >&2 <<'EOF'
compose: docker is required.

Install Docker Desktop:
  https://docs.docker.com/desktop/
EOF
  exit 1
fi

if ! docker compose version >/dev/null 2>&1; then
  cat >&2 <<'EOF'
compose: docker compose is required.

Install a Docker version with Compose V2 support:
  https://docs.docker.com/compose/
EOF
  exit 1
fi

if ! docker info >/dev/null 2>&1; then
  cat >&2 <<'EOF'
compose: Docker daemon is not available.

Start Docker Desktop, then retry the script.
EOF
  exit 1
fi

if [ "$#" -eq 0 ]; then
  usage
  exit 2
fi

command_name=$1
shift

cd "$repo_root"

case "$command_name" in
  up)
    exec docker compose up -d --wait mysql
    ;;
  down)
    exec docker compose down
    ;;
  reset)
    docker compose down --volumes
    exec docker compose up -d --wait mysql
    ;;
  ps)
    exec docker compose ps "$@"
    ;;
  logs)
    exec docker compose logs "$@"
    ;;
  exec)
    exec docker compose exec "$@"
    ;;
  *)
    usage
    exit 2
    ;;
esac
