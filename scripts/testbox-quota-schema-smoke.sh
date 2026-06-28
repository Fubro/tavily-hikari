#!/usr/bin/env bash
set -Eeuo pipefail

REMOTE_RUN="${REMOTE_RUN:?REMOTE_RUN is required}"
REMOTE_REPO_DIR="${REMOTE_REPO_DIR:?REMOTE_REPO_DIR is required}"
REMOTE_DB_DIR="${REMOTE_DB_DIR:?REMOTE_DB_DIR is required}"

SSH_TARGET="${SSH_TARGET:-codex-testbox}"
ADMIN_PASSWORD="${ADMIN_PASSWORD:-test-admin-password}"
ADMIN_BIND="${ADMIN_BIND:-127.0.0.1}"
ADMIN_PORT="${ADMIN_PORT:-18787}"
ADMIN_BASE_URL="http://${ADMIN_BIND}:${ADMIN_PORT}"
REPORT_PATH="${REPORT_PATH:-${REMOTE_RUN}/quota-schema-smoke-report.txt}"

ssh -o BatchMode=yes "${SSH_TARGET}" "bash -s" -- \
  "${REMOTE_RUN}" \
  "${REMOTE_REPO_DIR}" \
  "${REMOTE_DB_DIR}" \
  "${ADMIN_PASSWORD}" \
  "${ADMIN_BIND}" \
  "${ADMIN_PORT}" \
  "${REPORT_PATH}" <<'EOS'
set -Eeuo pipefail

REMOTE_RUN="$1"
REMOTE_REPO_DIR="$2"
REMOTE_DB_DIR="$3"
ADMIN_PASSWORD="$4"
ADMIN_BIND="$5"
ADMIN_PORT="$6"
REPORT_PATH="$7"
ADMIN_BASE_URL="http://${ADMIN_BIND}:${ADMIN_PORT}"
DB_PATH="${REMOTE_DB_DIR}/tavily_proxy.db"
OBS_DB_PATH="${REMOTE_DB_DIR}/tavily_proxy-observability.db"
LOG_PATH="${REMOTE_RUN}/quota-schema-smoke-server.log"
BUILD_LOG_PATH="${REMOTE_RUN}/quota-schema-smoke-build.log"
PID_PATH="${REMOTE_RUN}/quota-schema-smoke-server.pid"
ADMIN_COOKIE_JAR="${REMOTE_RUN}/quota-schema-admin.cookies"
USER_COOKIE_FILE="${REMOTE_RUN}/quota-schema-user.cookie"

write_user_session_with_retry() {
  local db_path="$1"
  local user_id="$2"
  local provider="$3"
  local session_token="$4"
  local expires_at="$5"
  python3 - "${db_path}" "${user_id}" "${provider}" "${session_token}" "${expires_at}" <<'PY'
import sqlite3
import sys
import time

db_path, user_id, provider, session_token, expires_at = sys.argv[1:6]
deadline = time.time() + 60
last_error = None

while time.time() < deadline:
    conn = None
    try:
        conn = sqlite3.connect(db_path, timeout=5)
        conn.execute("PRAGMA busy_timeout = 5000")
        conn.execute("BEGIN IMMEDIATE")
        conn.execute("DELETE FROM user_sessions WHERE user_id = ?", (user_id,))
        conn.execute(
            """
            INSERT INTO user_sessions (token, user_id, provider, created_at, expires_at, revoked_at)
            VALUES (?, ?, ?, strftime('%s','now'), ?, NULL)
            """,
            (session_token, user_id, provider, int(expires_at)),
        )
        conn.commit()
        print("user_session_write=ok")
        raise SystemExit(0)
    except sqlite3.OperationalError as exc:
        last_error = str(exc)
        if conn is not None:
            conn.rollback()
        time.sleep(1)
    finally:
        if conn is not None:
            conn.close()

raise SystemExit(f"failed to write user session after retries: {last_error}")
PY
}

cleanup() {
  if [[ -f "${PID_PATH}" ]]; then
    pid="$(cat "${PID_PATH}")"
    if kill -0 "${pid}" >/dev/null 2>&1; then
      kill "${pid}" >/dev/null 2>&1 || true
      wait "${pid}" >/dev/null 2>&1 || true
    fi
  fi
}
trap cleanup EXIT

cd "${REMOTE_REPO_DIR}"

{
  echo "== quota schema smoke =="
  echo "remote_run=${REMOTE_RUN}"
  echo "remote_repo_dir=${REMOTE_REPO_DIR}"
  echo "remote_db_dir=${REMOTE_DB_DIR}"
  echo "started_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
} > "${REPORT_PATH}"

{
  echo "== build current branch =="
  cargo build --release --locked --bin tavily-hikari
} >"${BUILD_LOG_PATH}" 2>&1

touch "${ADMIN_COOKIE_JAR}"

PROXY_DB_PATH="${DB_PATH}" \
WEB_STATIC_DIR="${REMOTE_REPO_DIR}/web/dist" \
PROXY_BIND="${ADMIN_BIND}" \
PROXY_PORT="${ADMIN_PORT}" \
ADMIN_AUTH_FORWARD_ENABLED=false \
ADMIN_AUTH_BUILTIN_ENABLED=true \
ADMIN_AUTH_BUILTIN_PASSWORD="${ADMIN_PASSWORD}" \
LINUXDO_OAUTH_ENABLED=true \
LINUXDO_OAUTH_CLIENT_ID=test-client-id \
LINUXDO_OAUTH_CLIENT_SECRET=test-client-secret \
LINUXDO_OAUTH_REDIRECT_URL=http://127.0.0.1/callback \
TAVILY_USAGE_BASE=http://127.0.0.1:9 \
./target/release/tavily-hikari >"${LOG_PATH}" 2>&1 &
SERVER_PID="$!"
echo "${SERVER_PID}" > "${PID_PATH}"

HEALTH_STATUS=""
HEALTH_BODY=""
for _ in $(seq 1 120); do
  HEALTH_STATUS="$(curl -sS -o "${REMOTE_RUN}/health-body.txt" -w '%{http_code}' "${ADMIN_BASE_URL}/health" || true)"
  HEALTH_BODY="$(cat "${REMOTE_RUN}/health-body.txt" 2>/dev/null || true)"
  if [[ "${HEALTH_STATUS}" == "200" || "${HEALTH_STATUS}" == "503" ]]; then
    break
  fi
  sleep 1
done

if [[ "${HEALTH_STATUS}" != "200" && "${HEALTH_STATUS}" != "503" ]]; then
  echo "unexpected /health status: ${HEALTH_STATUS}" >&2
  exit 1
fi

VERSION_JSON="$(curl -fsS "${ADMIN_BASE_URL}/api/version")"

{
  echo
  echo "== manifest =="
  cat "${REMOTE_DB_DIR}/manifest.env"
  echo
  echo "== sha256sums =="
  cat "${REMOTE_DB_DIR}/sha256sums.txt"
  echo
  echo "== health =="
  echo "status=${HEALTH_STATUS}"
  echo "body=${HEALTH_BODY}"
  echo
  echo "== schema account_quota_limits =="
  sqlite3 "${DB_PATH}" "SELECT name FROM pragma_table_info('account_quota_limits');"
  echo
  echo "== schema account_quota_limit_snapshots =="
  sqlite3 "${DB_PATH}" "SELECT name FROM pragma_table_info('account_quota_limit_snapshots');"
  echo
  echo "== schema user_tags =="
  sqlite3 "${DB_PATH}" "SELECT name FROM pragma_table_info('user_tags');"
} >> "${REPORT_PATH}"

python3 - "${DB_PATH}" <<'PY'
import sqlite3
import sys

db_path = sys.argv[1]
conn = sqlite3.connect(db_path)
checks = {
    "account_quota_limits": {
        "must_have": {"business_calls_1h_limit", "daily_credits_limit", "monthly_credits_limit"},
        "must_not_have": {"hourly_any_limit", "hourly_limit", "daily_limit", "monthly_limit"},
    },
    "account_quota_limit_snapshots": {
        "must_have": {"business_calls_1h_limit", "daily_credits_limit", "monthly_credits_limit"},
        "must_not_have": {"hourly_any_limit", "hourly_limit", "daily_limit", "monthly_limit"},
    },
    "user_tags": {
        "must_have": {"business_calls_1h_delta", "daily_credits_delta", "monthly_credits_delta"},
        "must_not_have": {"hourly_any_delta", "hourly_delta", "daily_delta", "monthly_delta"},
    },
}
for table, rule in checks.items():
    cols = {row[1] for row in conn.execute(f"PRAGMA table_info('{table}')")}
    missing = sorted(rule["must_have"] - cols)
    legacy = sorted(rule["must_not_have"] & cols)
    if missing or legacy:
        raise SystemExit(
            f"{table} schema mismatch: missing={missing or '-'} legacy_present={legacy or '-'}"
        )
print("schema_checks=ok")
PY

curl -fsS -c "${ADMIN_COOKIE_JAR}" \
  -H 'content-type: application/json' \
  -d "{\"password\":\"${ADMIN_PASSWORD}\"}" \
  "${ADMIN_BASE_URL}/api/admin/login" >/dev/null

ADMIN_USERS_JSON="${REMOTE_RUN}/admin-users.json"
ADMIN_DETAIL_JSON="${REMOTE_RUN}/admin-user-detail.json"
USER_DASHBOARD_JSON="${REMOTE_RUN}/user-dashboard.json"

curl -fsS -b "${ADMIN_COOKIE_JAR}" \
  "${ADMIN_BASE_URL}/api/users?page=1&per_page=1" > "${ADMIN_USERS_JSON}"

USER_ID="$(python3 - "${ADMIN_USERS_JSON}" <<'PY'
import json
import sys
with open(sys.argv[1], "r", encoding="utf-8") as fh:
    data = json.load(fh)
items = data.get("items") or []
if not items:
    raise SystemExit("no users returned from admin list")
print(items[0]["userId"])
PY
)"

curl -fsS -b "${ADMIN_COOKIE_JAR}" \
  "${ADMIN_BASE_URL}/api/users/${USER_ID}" > "${ADMIN_DETAIL_JSON}"

USER_SESSION_TOKEN="$(python3 - <<'PY'
import secrets
print(secrets.token_urlsafe(36))
PY
)"
USER_PROVIDER="$(sqlite3 "${DB_PATH}" "SELECT provider FROM oauth_accounts WHERE user_id = '${USER_ID}' ORDER BY updated_at DESC, created_at DESC LIMIT 1;")"
if [[ -z "${USER_PROVIDER}" ]]; then
  USER_PROVIDER="linuxdo"
fi
USER_EXPIRES_AT="$(python3 - <<'PY'
import time
print(int(time.time()) + 3600)
PY
)"
write_user_session_with_retry \
  "${DB_PATH}" \
  "${USER_ID}" \
  "${USER_PROVIDER}" \
  "${USER_SESSION_TOKEN}" \
  "${USER_EXPIRES_AT}" >> "${REPORT_PATH}"
printf 'hikari_user_session=%s\n' "${USER_SESSION_TOKEN}" > "${USER_COOKIE_FILE}"

curl -fsS \
  -H "Cookie: $(cat "${USER_COOKIE_FILE}")" \
  "${ADMIN_BASE_URL}/api/user/dashboard" > "${USER_DASHBOARD_JSON}"

python3 - "${ADMIN_USERS_JSON}" "${ADMIN_DETAIL_JSON}" "${USER_DASHBOARD_JSON}" "${VERSION_JSON}" <<'PY' >> "${REPORT_PATH}"
import json
import sys

admin_users_path, admin_detail_path, user_dashboard_path, version_json = sys.argv[1:5]
with open(admin_users_path, "r", encoding="utf-8") as fh:
    admin_users = json.load(fh)
with open(admin_detail_path, "r", encoding="utf-8") as fh:
    admin_detail = json.load(fh)
with open(user_dashboard_path, "r", encoding="utf-8") as fh:
    user_dashboard = json.load(fh)
version = json.loads(version_json)

def require_metric(payload, payload_name):
    missing = []
    if "requestRate" not in payload:
        missing.append("requestRate")
    if "businessCalls1h" not in payload:
        missing.append("businessCalls1h")
    if "dailyCreditsUsed" not in payload or "dailyCreditsLimit" not in payload:
        missing.append("dailyCredits")
    if "monthlyCreditsUsed" not in payload or "monthlyCreditsLimit" not in payload:
        missing.append("monthlyCredits")
    legacy = [key for key in payload if key.startswith("hourlyAny") or key.startswith("quotaHourly") or key.startswith("quotaDaily") or key.startswith("quotaMonthly")]
    if missing or legacy:
        raise SystemExit(f"{payload_name} contract mismatch: missing={missing or '-'} legacy={legacy or '-'}")

items = admin_users.get("items") or []
if not items:
    raise SystemExit("admin users response empty")
require_metric(items[0], "admin_users.items[0]")
require_metric(admin_detail, "admin_user_detail")
require_metric(user_dashboard, "user_dashboard")

print()
print("== api smoke ==")
print(f"version={version.get('version')}")
print(f"admin_user_id={items[0].get('userId')}")
print(f"admin_request_rate={items[0]['requestRate']}")
print(f"admin_business_calls_1h={items[0]['businessCalls1h']}")
print(f"user_request_rate={user_dashboard['requestRate']}")
print(f"user_business_calls_1h={user_dashboard['businessCalls1h']}")
print("contract_checks=ok")
PY

{
  echo
  echo "== server log tail =="
  tail -n 80 "${LOG_PATH}" || true
  echo
  echo "completed_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "status=ok"
} >> "${REPORT_PATH}"

cat "${REPORT_PATH}"
EOS
