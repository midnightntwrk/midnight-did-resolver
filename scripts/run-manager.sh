#!/usr/bin/env bash
set -euo pipefail

source ./scripts/run-common.sh
run_common_ensure_node
run_common_auto_proof_server_image "manager"

export DID_MANAGER_SETUP="${DID_MANAGER_SETUP:-standalone}"

echo "[manager] Build DID package prerequisites"
pnpm run build:did-prereqs

echo "[manager] Build secret-storage dependency"
pnpm --filter @midnight-ntwrk/midnight-did-secret-storage build

echo "[manager] Lint"
pnpm --filter @midnight-ntwrk/midnight-did-manager-service lint

echo "[manager] Build"
pnpm --filter @midnight-ntwrk/midnight-did-manager-service build

echo "[manager] Unit tests"
pnpm --filter @midnight-ntwrk/midnight-did-manager-service test

if [[ "${SKIP_LONG_RUNNING:-0}" == "1" ]]; then
  echo "[manager] Skip Playwright E2E (SKIP_LONG_RUNNING=1)"
else
  echo "[manager] Playwright install"
  env -u PLAYWRIGHT_BROWSERS_PATH pnpm --filter @midnight-ntwrk/midnight-did-manager-service playwright:install

  echo "[manager] Playwright E2E (standalone)"
  env -u PLAYWRIGHT_BROWSERS_PATH pnpm --filter @midnight-ntwrk/midnight-did-manager-service test:e2e:standalone
fi

echo "[manager] Done"
