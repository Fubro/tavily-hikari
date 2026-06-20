#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

DB_PATH="${DB_PATH:-tavily_proxy.db}"
RUN_UNTIL_COMPLETE="${RUN_UNTIL_COMPLETE:-true}"
JSON="${JSON:-true}"
COMPACT_AFTER="${COMPACT_AFTER:-false}"
FORCE_COMPACTION="${FORCE_COMPACTION:-false}"
BATCH_SIZE="${BATCH_SIZE:-20000}"
MAX_BATCHES="${MAX_BATCHES:-8}"
MAX_RUNTIME_SECS="${MAX_RUNTIME_SECS:-20}"
INTER_BATCH_SLEEP_MS="${INTER_BATCH_SLEEP_MS:-0}"

pushd "$ROOT_DIR" >/dev/null

cleanup_cmd=(
  cargo run --bin ha_outbox_cleanup_once --
  --db-path "$DB_PATH"
  --batch-size "$BATCH_SIZE"
  --max-batches "$MAX_BATCHES"
  --max-runtime-secs "$MAX_RUNTIME_SECS"
  --inter-batch-sleep-ms "$INTER_BATCH_SLEEP_MS"
)

if [[ "$RUN_UNTIL_COMPLETE" == "true" || "$RUN_UNTIL_COMPLETE" == "1" ]]; then
  cleanup_cmd+=(--run-until-complete)
fi

if [[ "$JSON" == "true" || "$JSON" == "1" ]]; then
  cleanup_cmd+=(--json)
fi

echo "Running HA outbox cleanup against $DB_PATH ..."
"${cleanup_cmd[@]}"

if [[ "$COMPACT_AFTER" == "true" || "$COMPACT_AFTER" == "1" ]]; then
  compaction_cmd=(cargo run --bin db_compaction_once -- --db-path "$DB_PATH")
  if [[ "$FORCE_COMPACTION" == "true" || "$FORCE_COMPACTION" == "1" ]]; then
    compaction_cmd+=(--force)
  fi
  if [[ "$JSON" == "true" || "$JSON" == "1" ]]; then
    compaction_cmd+=(--json)
  fi
  echo "Running SQLite compaction against $DB_PATH ..."
  "${compaction_cmd[@]}"
fi

popd >/dev/null
