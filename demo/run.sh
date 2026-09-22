#!/usr/bin/env bash
set -euo pipefail

DEMO_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE="${DEMO_DIR}/docker-compose.yml"
ENV_FILE="${DEMO_DIR}/.env"
PROJECT_NAME="${DEMO_PROJECT_NAME:-midnight-did-demo}"

compose_args=(-p "${PROJECT_NAME}" -f "${COMPOSE_FILE}")
if [[ -f "${ENV_FILE}" ]]; then
  compose_args+=(--env-file "${ENV_FILE}")
  # The example file contains simple KEY=VALUE assignments. Load the local
  # overrides as well so health URLs match the published host-port mappings.
  set -a
  # shellcheck disable=SC1090
  source "${ENV_FILE}"
  set +a
fi

if [[ -z "${MANAGER_DATA_DIR:-}" ]]; then
  if [[ -z "${HOME:-}" ]]; then
    echo "[demo] HOME must be set to persist manager state." >&2
    exit 1
  fi
  MANAGER_DATA_DIR="${HOME}/.midnight-did/manager"
fi
export MANAGER_DATA_DIR
export MANAGER_CONTAINER_UID="${MANAGER_CONTAINER_UID:-$(id -u)}"
export MANAGER_CONTAINER_GID="${MANAGER_CONTAINER_GID:-$(id -g)}"

usage() {
  cat <<'EOF'
Usage: ./demo/run.sh <command> [service...]

Commands:
  up       Start the resolver, manager, and local Midnight dependencies.
  down     Stop and remove the demo containers and network.
  clean    Stop containers and remove demo volumes/networks.
  health   Check resolver and manager health endpoints.
  logs     Follow logs (optionally for selected services).
  config   Render and validate the resolved Docker Compose configuration.

Configuration:
  Copy demo/.env.example to demo/.env to override image references or host
  ports. Manager state is persisted in ~/.midnight-did/manager by default.
  The default images are the 0.1.0-rc.1 GHCR release candidates.
EOF
}

require_docker() {
  if ! command -v docker >/dev/null 2>&1; then
    echo "[demo] Docker is required but was not found in PATH." >&2
    exit 1
  fi
  if ! docker compose version >/dev/null 2>&1; then
    echo "[demo] Docker Compose v2 is required (run: docker compose version)." >&2
    exit 1
  fi
}

compose() {
  docker compose "${compose_args[@]}" "$@"
}

prepare_manager_data_dir() {
  mkdir -p "${MANAGER_DATA_DIR}"
}

health_url() {
  local port="$1"
  printf 'http://127.0.0.1:%s/health\n' "${port}"
}

wait_for_health() {
  local name="$1"
  local url="$2"
  local deadline=$((SECONDS + 180))

  until curl --fail --silent --show-error "${url}" >/dev/null 2>&1; do
    if (( SECONDS >= deadline )); then
      echo "[demo] ${name} did not become healthy at ${url}" >&2
      return 1
    fi
    sleep 2
  done

  echo "[demo] ${name} healthy: ${url}"
}

health() {
  if ! command -v curl >/dev/null 2>&1; then
    echo "[demo] curl is required for health checks." >&2
    exit 1
  fi

  local resolver_port="${RESOLVER_PORT:-3001}"
  local manager_port="${MANAGER_PORT:-3010}"
  local resolver_url manager_url
  resolver_url="$(health_url "${resolver_port}")"
  manager_url="$(health_url "${manager_port}")"

  wait_for_health resolver "${resolver_url}"
  wait_for_health manager "${manager_url}"
}

require_docker

command_name="${1:-help}"
if [[ $# -gt 0 ]]; then
  shift
fi

case "${command_name}" in
  up)
    prepare_manager_data_dir
    compose up -d "$@"
    echo "[demo] resolver: http://127.0.0.1:${RESOLVER_PORT:-3001}"
    echo "[demo] manager:  http://127.0.0.1:${MANAGER_PORT:-3010}/wallet"
    ;;
  down)
    compose down --remove-orphans
    ;;
  clean)
    compose down --remove-orphans --volumes
    ;;
  health)
    health
    ;;
  logs)
    compose logs -f "$@"
    ;;
  config)
    compose config
    ;;
  help|-h|--help)
    usage
    ;;
  *)
    echo "[demo] Unknown command: ${command_name}" >&2
    usage >&2
    exit 1
    ;;
esac
