#!/usr/bin/env bash
# Wait until the Snowflake test-account hostname resolves.
#
# GitHub-hosted macOS runners intermittently return getaddrinfo EAI_NONAME
# ("nodename nor servname provided, or not known") for the test host. Login
# HTTP retries and nextest/pytest reruns are too short to cover that window,
# so live-account jobs wait here after decode_secrets.sh and before tests.
set -euo pipefail

TIMEOUT_SECONDS="${WAIT_FOR_TEST_HOST_TIMEOUT_SECONDS:-90}"
INTERVAL_SECONDS="${WAIT_FOR_TEST_HOST_INTERVAL_SECONDS:-2}"
PARAMS="${1:-${PARAMETER_PATH:-parameters.json}}"

if command -v python3 >/dev/null 2>&1; then
  PY=python3
elif command -v python >/dev/null 2>&1; then
  PY=python
else
  echo "wait_for_test_host: python3 (or python) is required to parse parameters.json and resolve DNS" >&2
  exit 2
fi

if [[ ! -f "${PARAMS}" ]]; then
  echo "wait_for_test_host: parameters file not found: ${PARAMS}" >&2
  exit 2
fi

HOST="$(
  "${PY}" - "${PARAMS}" <<'PY'
import json
import sys
from pathlib import Path

data = json.loads(Path(sys.argv[1]).read_text())
conn = data.get("testconnection") or {}
host = str(conn.get("SNOWFLAKE_TEST_HOST") or "").strip()
if host:
    print(host)
    raise SystemExit(0)
account = str(conn.get("SNOWFLAKE_TEST_ACCOUNT") or "").strip()
if account:
    print(f"{account}.snowflakecomputing.com")
PY
)"

if [[ -z "${HOST}" ]]; then
  echo "wait_for_test_host: SNOWFLAKE_TEST_HOST and SNOWFLAKE_TEST_ACCOUNT are both empty in ${PARAMS}" >&2
  exit 2
fi

lookup() {
  local host="$1"
  if [[ -n "${WAIT_FOR_TEST_HOST_LOOKUP:-}" ]]; then
    "${WAIT_FOR_TEST_HOST_LOOKUP}" "${host}"
  else
    "${PY}" -c 'import socket, sys; socket.getaddrinfo(sys.argv[1], None)' "${host}"
  fi
}

start_s="$(date +%s)"
attempt=0
while true; do
  attempt=$((attempt + 1))
  if lookup "${HOST}" >/dev/null 2>&1; then
    echo "wait_for_test_host: resolved ${HOST} on attempt ${attempt}"
    exit 0
  fi

  now_s="$(date +%s)"
  elapsed=$((now_s - start_s))
  if (( elapsed >= TIMEOUT_SECONDS )); then
    echo "wait_for_test_host: runner DNS did not resolve test host ${HOST} after ${elapsed}s (${attempt} attempts)" >&2
    echo "wait_for_test_host: this is a runner DNS failure, not a Snowflake account outage" >&2
    exit 1
  fi

  echo "wait_for_test_host: ${HOST} not yet resolvable (attempt ${attempt}, ${elapsed}s); retrying in ${INTERVAL_SECONDS}s"
  sleep "${INTERVAL_SECONDS}"
done
