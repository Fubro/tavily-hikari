#!/usr/bin/env bash
set -euo pipefail

show_help() {
  cat <<'EOF'
Usage: scripts/export-live-db-snapshot-to-testbox.sh

Create a full read-only SQLite snapshot set on machine 101 and upload it into an isolated
codex-testbox run directory for offline validation.

Environment variables:
  SOURCE_HOST                 Defaults to 192.168.31.11
  SOURCE_SSH_TARGET           Defaults to SOURCE_HOST
  TESTBOX_HOST                Defaults to codex-testbox
  SOURCE_DB_DIR               Defaults to /var/lib/docker/volumes/ai-tavily-hikari-data/_data
  SOURCE_CORE_DB_NAME         Defaults to tavily_proxy.db
  SOURCE_OBSERVABILITY_DB_NAME Defaults to tavily_proxy-observability.db
  SOURCE_SNAPSHOT_DIR         Defaults to /tmp/<repo>-<run-id>
  RUN_ID                      Optional explicit run id
  KEEP_SOURCE_SNAPSHOTS       true/false, defaults to false

Outputs:
  Prints the remote run directory and writes manifests under:
    <remote-run>/live-db/manifest.env
    <remote-run>/live-db/sha256sums.txt
EOF
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  show_help
  exit 0
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE_HOST="${SOURCE_HOST:-192.168.31.11}"
SOURCE_SSH_TARGET="${SOURCE_SSH_TARGET:-$SOURCE_HOST}"
TESTBOX_HOST="${TESTBOX_HOST:-codex-testbox}"
SOURCE_DB_DIR="${SOURCE_DB_DIR:-/var/lib/docker/volumes/ai-tavily-hikari-data/_data}"
SOURCE_CORE_DB_NAME="${SOURCE_CORE_DB_NAME:-tavily_proxy.db}"
SOURCE_OBSERVABILITY_DB_NAME="${SOURCE_OBSERVABILITY_DB_NAME:-tavily_proxy-observability.db}"
KEEP_SOURCE_SNAPSHOTS="${KEEP_SOURCE_SNAPSHOTS:-false}"

if REPO_ROOT="$(git -C "$ROOT_DIR" rev-parse --show-toplevel 2>/dev/null)"; then
  :
else
  REPO_ROOT="$ROOT_DIR"
fi
REPO_ROOT="$(python3 - "$REPO_ROOT" <<'PY'
import os
import sys
print(os.path.realpath(sys.argv[1]))
PY
)"

REPO_NAME="$(basename "$REPO_ROOT")"
PATH_HASH8="$(python3 - "$REPO_ROOT" <<'PY'
import hashlib
import os
import sys
path = os.path.realpath(sys.argv[1]).encode()
print(hashlib.sha256(path).hexdigest()[:8])
PY
)"
GIT_SHA="$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo nogit)"
RUN_ID="${RUN_ID:-$(date -u +%Y%m%d_%H%M%S)_${GIT_SHA}_ha_outbox}"
WORKSPACE_SLUG="${REPO_NAME}__${PATH_HASH8}"
REMOTE_BASE="/srv/codex/workspaces/$USER"
REMOTE_WORKSPACE="$REMOTE_BASE/$WORKSPACE_SLUG"
REMOTE_RUN="$REMOTE_WORKSPACE/runs/$RUN_ID"
REMOTE_REPO_DIR="$REMOTE_RUN/repo"
REMOTE_DB_DIR="$REMOTE_RUN/live-db"

SOURCE_TMP_DIR="${SOURCE_SNAPSHOT_DIR:-/tmp/${REPO_NAME}-${RUN_ID}}"
SOURCE_CORE_LIVE="$SOURCE_DB_DIR/$SOURCE_CORE_DB_NAME"
SOURCE_SIDECAR_LIVE="$SOURCE_DB_DIR/$SOURCE_OBSERVABILITY_DB_NAME"
SOURCE_CORE_SNAPSHOT="$SOURCE_TMP_DIR/$SOURCE_CORE_DB_NAME"
SOURCE_SIDECAR_SNAPSHOT="$SOURCE_TMP_DIR/$SOURCE_OBSERVABILITY_DB_NAME"

manifest_get() {
  local key="$1"
  printf '%s\n' "$SOURCE_MANIFEST" | awk -F= -v target="$key" '$1 == target { sub($1"=",""); print; exit }'
}

printf 'Preparing isolated codex-testbox run dir: %s\n' "$REMOTE_RUN"
ssh -o BatchMode=yes "$TESTBOX_HOST" "mkdir -p '$REMOTE_DB_DIR' '$REMOTE_REPO_DIR' '$REMOTE_WORKSPACE'"

CREATED_UTC="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
ssh -o BatchMode=yes "$TESTBOX_HOST" "cat > '$REMOTE_WORKSPACE/workspace.txt'" <<TXT
local_repo_root=$REPO_ROOT
created_utc=$CREATED_UTC
source_host=$SOURCE_HOST
source_db_dir=$SOURCE_DB_DIR
TXT

printf 'Syncing repo to codex-testbox run dir...\n'
rsync -az --delete \
  --exclude '.git/' \
  --exclude 'node_modules/' \
  --exclude 'target/' \
  --exclude 'dist/' \
  --exclude 'build/' \
  --exclude '.next/' \
  --exclude '.venv/' \
  --exclude '*.db' \
  --exclude '*.db-*' \
  "$REPO_ROOT/" "$TESTBOX_HOST:$REMOTE_REPO_DIR/"

printf 'Creating read-only SQLite backups on %s ...\n' "$SOURCE_SSH_TARGET"
SOURCE_MANIFEST="$(ssh -o BatchMode=yes "$SOURCE_SSH_TARGET" "bash -s" -- \
  "$SOURCE_TMP_DIR" \
  "$SOURCE_CORE_LIVE" \
  "$SOURCE_SIDECAR_LIVE" \
  "$SOURCE_CORE_SNAPSHOT" \
  "$SOURCE_SIDECAR_SNAPSHOT" <<'EOS'
set -euo pipefail

tmp_dir="$1"
core_live="$2"
sidecar_live="$3"
core_snapshot="$4"
sidecar_snapshot="$5"

mkdir -p "$tmp_dir"
test -f "$core_live"
test -f "$sidecar_live"

core_live_bytes="$(stat -c %s "$core_live")"
core_live_wal_bytes="$(stat -c %s "${core_live}-wal" 2>/dev/null || echo 0)"
sidecar_live_bytes="$(stat -c %s "$sidecar_live")"
available_tmp_bytes="$(df -B1 --output=avail "$tmp_dir" | tail -n1 | tr -d ' ')"
required_tmp_bytes="$((core_live_bytes + core_live_wal_bytes + sidecar_live_bytes + 1073741824))"

if (( available_tmp_bytes < required_tmp_bytes )); then
  echo "insufficient temporary free space for snapshot: available=${available_tmp_bytes} required=${required_tmp_bytes}" >&2
  exit 2
fi

rm -f "$core_snapshot" "$sidecar_snapshot"
sqlite3 "$core_live" ".timeout 10000" ".backup '$core_snapshot'"
sqlite3 "$sidecar_live" ".timeout 10000" ".backup '$sidecar_snapshot'"

core_integrity="$(sqlite3 "$core_snapshot" 'PRAGMA integrity_check;' | tr -d '\r')"
sidecar_integrity="$(sqlite3 "$sidecar_snapshot" 'PRAGMA integrity_check;' | tr -d '\r')"

if [[ "$core_integrity" != "ok" ]]; then
  echo "core snapshot integrity_check failed: $core_integrity" >&2
  exit 3
fi
if [[ "$sidecar_integrity" != "ok" ]]; then
  echo "sidecar snapshot integrity_check failed: $sidecar_integrity" >&2
  exit 4
fi

printf 'source_tmp_dir=%s\n' "$tmp_dir"
printf 'core_live_path=%s\n' "$core_live"
printf 'sidecar_live_path=%s\n' "$sidecar_live"
printf 'core_live_bytes=%s\n' "$core_live_bytes"
printf 'core_live_wal_bytes=%s\n' "$core_live_wal_bytes"
printf 'sidecar_live_bytes=%s\n' "$sidecar_live_bytes"
printf 'available_tmp_bytes=%s\n' "$available_tmp_bytes"
printf 'required_tmp_bytes=%s\n' "$required_tmp_bytes"
printf 'core_snapshot_path=%s\n' "$core_snapshot"
printf 'sidecar_snapshot_path=%s\n' "$sidecar_snapshot"
printf 'core_snapshot_bytes=%s\n' "$(stat -c %s "$core_snapshot")"
printf 'sidecar_snapshot_bytes=%s\n' "$(stat -c %s "$sidecar_snapshot")"
printf 'core_snapshot_sha256=%s\n' "$(sha256sum "$core_snapshot" | awk '{print $1}')"
printf 'sidecar_snapshot_sha256=%s\n' "$(sha256sum "$sidecar_snapshot" | awk '{print $1}')"
printf 'core_snapshot_integrity=%s\n' "$core_integrity"
printf 'sidecar_snapshot_integrity=%s\n' "$sidecar_integrity"
EOS
)"

SOURCE_TMP_DIR_REMOTE="$(manifest_get source_tmp_dir)"
CORE_LIVE_PATH_REMOTE="$(manifest_get core_live_path)"
SIDECAR_LIVE_PATH_REMOTE="$(manifest_get sidecar_live_path)"
CORE_LIVE_BYTES="$(manifest_get core_live_bytes)"
CORE_LIVE_WAL_BYTES="$(manifest_get core_live_wal_bytes)"
SIDECAR_LIVE_BYTES="$(manifest_get sidecar_live_bytes)"
AVAILABLE_TMP_BYTES="$(manifest_get available_tmp_bytes)"
REQUIRED_TMP_BYTES="$(manifest_get required_tmp_bytes)"
CORE_SNAPSHOT_PATH_REMOTE="$(manifest_get core_snapshot_path)"
SIDECAR_SNAPSHOT_PATH_REMOTE="$(manifest_get sidecar_snapshot_path)"
CORE_SNAPSHOT_BYTES="$(manifest_get core_snapshot_bytes)"
SIDECAR_SNAPSHOT_BYTES="$(manifest_get sidecar_snapshot_bytes)"
CORE_SNAPSHOT_SHA256="$(manifest_get core_snapshot_sha256)"
SIDECAR_SNAPSHOT_SHA256="$(manifest_get sidecar_snapshot_sha256)"
CORE_SNAPSHOT_INTEGRITY="$(manifest_get core_snapshot_integrity)"
SIDECAR_SNAPSHOT_INTEGRITY="$(manifest_get sidecar_snapshot_integrity)"

printf 'Streaming full snapshot set to codex-testbox ...\n'
ssh -o BatchMode=yes "$SOURCE_SSH_TARGET" "cat '$CORE_SNAPSHOT_PATH_REMOTE'" \
  | ssh -o BatchMode=yes "$TESTBOX_HOST" "cat > '$REMOTE_DB_DIR/$SOURCE_CORE_DB_NAME'"
ssh -o BatchMode=yes "$SOURCE_SSH_TARGET" "cat '$SIDECAR_SNAPSHOT_PATH_REMOTE'" \
  | ssh -o BatchMode=yes "$TESTBOX_HOST" "cat > '$REMOTE_DB_DIR/$SOURCE_OBSERVABILITY_DB_NAME'"

ssh -o BatchMode=yes "$TESTBOX_HOST" "cat > '$REMOTE_DB_DIR/manifest.env'" <<EOF
run_id=$RUN_ID
created_utc=$CREATED_UTC
source_host=$SOURCE_HOST
source_ssh_target=$SOURCE_SSH_TARGET
source_db_dir=$SOURCE_DB_DIR
core_live_path=$CORE_LIVE_PATH_REMOTE
sidecar_live_path=$SIDECAR_LIVE_PATH_REMOTE
core_live_bytes=$CORE_LIVE_BYTES
core_live_wal_bytes=$CORE_LIVE_WAL_BYTES
sidecar_live_bytes=$SIDECAR_LIVE_BYTES
available_tmp_bytes=$AVAILABLE_TMP_BYTES
required_tmp_bytes=$REQUIRED_TMP_BYTES
core_snapshot_bytes=$CORE_SNAPSHOT_BYTES
sidecar_snapshot_bytes=$SIDECAR_SNAPSHOT_BYTES
core_snapshot_sha256=$CORE_SNAPSHOT_SHA256
sidecar_snapshot_sha256=$SIDECAR_SNAPSHOT_SHA256
core_snapshot_integrity=$CORE_SNAPSHOT_INTEGRITY
sidecar_snapshot_integrity=$SIDECAR_SNAPSHOT_INTEGRITY
remote_run=$REMOTE_RUN
remote_repo_dir=$REMOTE_REPO_DIR
remote_db_dir=$REMOTE_DB_DIR
EOF

ssh -o BatchMode=yes "$TESTBOX_HOST" "cat > '$REMOTE_DB_DIR/sha256sums.txt'" <<EOF
$CORE_SNAPSHOT_SHA256  $SOURCE_CORE_DB_NAME
$SIDECAR_SNAPSHOT_SHA256  $SOURCE_OBSERVABILITY_DB_NAME
EOF

printf 'Verifying uploaded files on codex-testbox ...\n'
ssh -o BatchMode=yes "$TESTBOX_HOST" "cd '$REMOTE_DB_DIR' \
  && sha256sum -c sha256sums.txt \
  && test \"\$(sqlite3 '$SOURCE_CORE_DB_NAME' 'PRAGMA integrity_check;')\" = ok \
  && test \"\$(sqlite3 '$SOURCE_OBSERVABILITY_DB_NAME' 'PRAGMA integrity_check;')\" = ok"

if [[ "$KEEP_SOURCE_SNAPSHOTS" != "true" && "$KEEP_SOURCE_SNAPSHOTS" != "1" ]]; then
  printf 'Cleaning temporary backups on %s ...\n' "$SOURCE_SSH_TARGET"
  ssh -o BatchMode=yes "$SOURCE_SSH_TARGET" "rm -f '$CORE_SNAPSHOT_PATH_REMOTE' '$SIDECAR_SNAPSHOT_PATH_REMOTE' && rmdir '$SOURCE_TMP_DIR_REMOTE' 2>/dev/null || true"
fi

printf '\nSnapshot export complete.\n'
printf 'REMOTE_RUN=%s\n' "$REMOTE_RUN"
printf 'REMOTE_REPO_DIR=%s\n' "$REMOTE_REPO_DIR"
printf 'REMOTE_DB_DIR=%s\n' "$REMOTE_DB_DIR"
printf 'CORE_SHA256=%s\n' "$CORE_SNAPSHOT_SHA256"
printf 'SIDECAR_SHA256=%s\n' "$SIDECAR_SNAPSHOT_SHA256"
