#!/usr/bin/env bash
set -euo pipefail

node ./scripts/ensure-node-24.mjs

echo "[docs] Build VitePress site"
pnpm run docs:build

if [[ "${DOCS_PREVIEW:-0}" == "1" ]]; then
  echo "[docs] Preview built site"
  pnpm run docs:preview
else
  echo "[docs] Done"
  echo "[docs] Dev server: pnpm run docs:dev"
  echo "[docs] Preview built site: DOCS_PREVIEW=1 ./scripts/run-docs.sh"
fi
