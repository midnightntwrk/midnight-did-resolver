#!/usr/bin/env bash
set -euo pipefail

source ./scripts/run-common.sh

run_common_parse_args "run" "$@"
run_common_warn_unsupported_flags "${RUN_COMMON_TARGET}"

case "${RUN_COMMON_TARGET}" in
  targets|help)
    run_common_usage "run"
    exit 0
    ;;
esac

run_catalog_steps() {
  local target_name="$1"
  local labels=()
  local commands=()
  local line
  local i

  while IFS= read -r line; do
    [[ -n "${line}" ]] && labels+=("${line}")
  done < <(run_common_catalog --step-labels "${target_name}")

  while IFS= read -r line; do
    [[ -n "${line}" ]] && commands+=("${line}")
  done < <(run_common_catalog --step-commands "${target_name}")

  if [[ "${#labels[@]}" == "0" || "${#labels[@]}" != "${#commands[@]}" ]]; then
    echo "[run] No executable runner steps found for target '${target_name}'" >&2
    exit 1
  fi

  for i in "${!labels[@]}"; do
    run_common_run_step "${labels[$i]}" "${commands[$i]}"
  done
}

run_common_ensure_node

if [[ "${SKIP_LONG_RUNNING:-0}" == "1" ]]; then
  echo "[run] Fast mode enabled: long-running integration/UI targets will be skipped"
fi

run_catalog_steps "${RUN_COMMON_TARGET}"

echo "All steps completed successfully."
run_common_finish
