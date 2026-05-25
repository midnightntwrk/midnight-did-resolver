#!/usr/bin/env bash
set -euo pipefail

source ./scripts/run-common.sh
run_common_ensure_node

echo "[secret-storage] Build DID Schnorr prerequisite"
pnpm run build:did-jubjub-schnorr

if [[ "${SKIP_LINT_FIX:-0}" == "1" ]]; then
  echo "[secret-storage] Lint"
  pnpm --filter @midnight-ntwrk/midnight-did-secret-storage lint
else
  echo "[secret-storage] Lint (fix)"
  pnpm --filter @midnight-ntwrk/midnight-did-secret-storage lint:fix || pnpm --filter @midnight-ntwrk/midnight-did-secret-storage lint
fi

echo "[secret-storage] Build"
pnpm --filter @midnight-ntwrk/midnight-did-secret-storage build

echo "[secret-storage] Test"
pnpm --filter @midnight-ntwrk/midnight-did-secret-storage test

echo "[secret-storage] Done"
