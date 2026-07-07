#!/usr/bin/env bash
set -euo pipefail

source ./scripts/run-common.sh
run_common_ensure_node
run_common_auto_proof_server_image "resolver"

echo "[resolver] Check DID package refs"
npm run build:did-prereqs

echo "[resolver] Build secret-storage dependency"
npm run build -w @midnight-ntwrk/midnight-did-secret-storage

echo "[resolver] Lint"
npm run lint -w @midnight-ntwrk/midnight-did-resolver-service

echo "[resolver] Build"
npm run build -w @midnight-ntwrk/midnight-did-resolver-service

echo "[resolver] Unit tests"
npm run test -w @midnight-ntwrk/midnight-did-resolver-service

if [[ "${SKIP_LONG_RUNNING:-0}" == "1" ]]; then
  echo "[resolver] Skip resolver integration tests (SKIP_LONG_RUNNING=1)"
else
  echo "[resolver] Integration tests"
  npm run test:integration -w @midnight-ntwrk/midnight-did-resolver-service
fi

echo "[resolver] Done"
