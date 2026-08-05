#!/usr/bin/env bash
set -euo pipefail

source ./scripts/run-common.sh
run_common_ensure_node

echo "[secret-storage] Check DID Schnorr package ref"
npm run build:did-jubjub-schnorr

if [[ "${SKIP_LINT_FIX:-0}" == "1" ]]; then
  echo "[secret-storage] Lint"
  npm run lint -w @midnight-ntwrk/midnight-did-secret-storage
else
  echo "[secret-storage] Lint (fix)"
  npm run lint:fix -w @midnight-ntwrk/midnight-did-secret-storage || npm run lint -w @midnight-ntwrk/midnight-did-secret-storage
fi

echo "[secret-storage] Build"
npm --ignore-scripts run build -w @midnight-ntwrk/midnight-did-secret-storage

echo "[secret-storage] Test"
npm --ignore-scripts run test -w @midnight-ntwrk/midnight-did-secret-storage

echo "[secret-storage] Done"
