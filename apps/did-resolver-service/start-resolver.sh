#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

# Local defaults; caller can override via environment.
export RESOLVER_HOST="${RESOLVER_HOST:-127.0.0.1}"
export RESOLVER_PORT="${RESOLVER_PORT:-3001}"
export MIDNIGHT_INDEXER_HTTP_URL="${MIDNIGHT_INDEXER_HTTP_URL:-http://127.0.0.1:8088/api/v3/graphql}"
export MIDNIGHT_INDEXER_WS_URL="${MIDNIGHT_INDEXER_WS_URL:-ws://127.0.0.1:8088/api/v3/graphql/ws}"

cd "$REPO_ROOT"

case "${1:-}" in
  --dev)
    # Keep explicit dev mode for local experiments; may require a TS runner
    # compatible with ESM .js specifier imports.
    pnpm --filter @midnight-ntwrk/midnight-did-resolver-service dev
    ;;
  --build|"")
    # Default path: compile TS and run dist for maximum compatibility.
    pnpm --filter @midnight-ntwrk/midnight-did-resolver-service build
    pnpm --filter @midnight-ntwrk/midnight-did-resolver-service start
    ;;
  *)
    echo "Usage: $0 [--build|--dev]" >&2
    exit 2
    ;;
esac
