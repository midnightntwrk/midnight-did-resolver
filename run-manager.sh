#!/usr/bin/env bash
set -euo pipefail

source ./scripts/run-common.sh
run_common_ensure_node
run_common_auto_proof_server_image "manager"

export DID_MANAGER_SETUP="${DID_MANAGER_SETUP:-standalone}"

echo "[manager] Build DID package prerequisites"
npm run build:did-prereqs

echo "[manager] Build secret-storage dependency"
npm run build -w @midnight-ntwrk/midnight-did-secret-storage

echo "[manager] Lint"
npm run lint -w @midnight-ntwrk/midnight-did-manager-service

echo "[manager] Build"
npm --ignore-scripts run build -w @midnight-ntwrk/midnight-did-manager-service

echo "[manager] Unit tests"
npm --ignore-scripts run test -w @midnight-ntwrk/midnight-did-manager-service

if [[ "${SKIP_LONG_RUNNING:-0}" == "1" ]]; then
  echo "[manager] Skip Playwright E2E (SKIP_LONG_RUNNING=1)"
else
  echo "[manager] Playwright install"
  env -u PLAYWRIGHT_BROWSERS_PATH npm run playwright:install -w @midnight-ntwrk/midnight-did-manager-service

  echo "[manager] Playwright E2E (standalone)"
  env -u PLAYWRIGHT_BROWSERS_PATH npm run test:e2e:standalone -w @midnight-ntwrk/midnight-did-manager-service
fi

echo "[manager] Done"
