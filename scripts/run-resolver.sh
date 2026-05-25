#!/usr/bin/env bash
set -euo pipefail

source ./scripts/run-common.sh
run_common_ensure_node
run_common_auto_proof_server_image "resolver"

echo "[resolver] Build DID package prerequisites"
pnpm run build:did-prereqs

echo "[resolver] Build secret-storage dependency"
pnpm --filter @midnight-ntwrk/midnight-did-secret-storage build

echo "[resolver] Lint"
pnpm --filter @midnight-ntwrk/midnight-did-resolver-service lint

echo "[resolver] Build"
pnpm --filter @midnight-ntwrk/midnight-did-resolver-service build

echo "[resolver] Unit tests"
pnpm --filter @midnight-ntwrk/midnight-did-resolver-service test

if [[ "${SKIP_LONG_RUNNING:-0}" == "1" ]]; then
  echo "[resolver] Skip resolver integration tests (SKIP_LONG_RUNNING=1)"
else
  echo "[resolver] Integration tests"
  pnpm --filter @midnight-ntwrk/midnight-did-resolver-service test:integration
fi

echo "[resolver] Done"
