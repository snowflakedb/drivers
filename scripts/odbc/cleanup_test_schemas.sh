#!/usr/bin/env bash
# Drop orphaned TEMP_TEST_SCHEMA_* schemas older than N days.
#
# Safety net for schemas left behind by crashed/killed test processes.
# Uses curl + jq against the Snowflake REST API (no extra dependencies).
#
# Required env: PARAMETER_PATH (path to parameters.json)
# Optional env: SNOWFLAKE_CLEANUP_AGE_DAYS (default: 2)

set -euo pipefail

AGE_DAYS="${SNOWFLAKE_CLEANUP_AGE_DAYS:-2}"
PARAM_PATH="${PARAMETER_PATH:-}"
TAG="cleanup_test_schemas"

if [[ -z "$PARAM_PATH" || ! -f "$PARAM_PATH" ]]; then
    echo "$TAG: PARAMETER_PATH not set or file missing, skipping"
    exit 0
fi

for cmd in jq curl; do
    if ! command -v "$cmd" &>/dev/null; then
        echo "$TAG: $cmd not found, skipping"
        exit 0
    fi
done

ACCOUNT=$(jq -r '.testconnection.SNOWFLAKE_TEST_ACCOUNT // empty' "$PARAM_PATH")
USER=$(jq -r '.testconnection.SNOWFLAKE_TEST_USER // empty' "$PARAM_PATH")
PASSWORD=$(jq -r '.testconnection.SNOWFLAKE_TEST_PASSWORD // empty' "$PARAM_PATH")
DATABASE=$(jq -r '.testconnection.SNOWFLAKE_TEST_DATABASE // empty' "$PARAM_PATH")
WAREHOUSE=$(jq -r '.testconnection.SNOWFLAKE_TEST_WAREHOUSE // empty' "$PARAM_PATH")
HOST=$(jq -r '.testconnection.SNOWFLAKE_TEST_HOST // empty' "$PARAM_PATH")

if [[ -z "$ACCOUNT" || -z "$USER" || -z "$PASSWORD" ]]; then
    echo "$TAG: missing account/user/password in parameters, skipping"
    exit 0
fi
if [[ -z "$DATABASE" ]]; then
    echo "$TAG: no SNOWFLAKE_TEST_DATABASE configured, skipping"
    exit 0
fi

if [[ -n "$HOST" ]]; then
    BASE_URL="https://${HOST}"
else
    BASE_URL="https://${ACCOUNT}.snowflakecomputing.com"
fi

# --- Authenticate via login-request to obtain a session token ----------------
LOGIN_BODY=$(jq -n \
    --arg acct "$ACCOUNT" \
    --arg user "$USER" \
    --arg pass "$PASSWORD" \
    '{data: {ACCOUNT_NAME: $acct, LOGIN_NAME: $user, PASSWORD: $pass,
             CLIENT_APP_ID: "cleanup_test_schemas", CLIENT_APP_VERSION: "1.0"}}')

LOGIN_RESP=$(echo "$LOGIN_BODY" | curl -sf -X POST "${BASE_URL}/session/v1/login-request" \
    -H "Content-Type: application/json" \
    --data-binary @- 2>/dev/null) || {
    echo "$TAG: login request failed, skipping"
    exit 0
}

TOKEN=$(echo "$LOGIN_RESP" | jq -r '.data.token // empty')
if [[ -z "$TOKEN" ]]; then
    echo "$TAG: authentication failed (no token), skipping"
    exit 0
fi

AUTH_HEADER="Authorization: Snowflake Token=\"${TOKEN}\""

run_sql() {
    local sql="$1"
    local body
    if [[ -n "$WAREHOUSE" ]]; then
        body=$(jq -n \
            --arg stmt "$sql" \
            --arg db "$DATABASE" \
            --arg wh "$WAREHOUSE" \
            '{statement: $stmt, database: $db, warehouse: $wh, timeout: 30}')
    else
        body=$(jq -n \
            --arg stmt "$sql" \
            --arg db "$DATABASE" \
            '{statement: $stmt, database: $db, timeout: 30}')
    fi

    echo "$body" | curl -sf -X POST "${BASE_URL}/api/v2/statements" \
        -H "Content-Type: application/json" \
        -H "$AUTH_HEADER" \
        --data-binary @- 2>/dev/null
}

logout() {
    curl -sf -X POST "${BASE_URL}/session" \
        -H "Content-Type: application/json" \
        -H "$AUTH_HEADER" \
        -d '{"action":"delete"}' >/dev/null 2>&1 || true
}
trap logout EXIT

# --- Query for orphaned schemas ----------------------------------------------
echo "$TAG: looking for TEMP_TEST_SCHEMA_% older than ${AGE_DAYS} days in ${DATABASE}"

QUERY="SELECT SCHEMA_NAME FROM ${DATABASE}.INFORMATION_SCHEMA.SCHEMATA \
WHERE SCHEMA_NAME LIKE 'TEMP_TEST_SCHEMA_%' \
AND CREATED < DATEADD(day, -${AGE_DAYS}, CURRENT_TIMESTAMP()) \
ORDER BY CREATED"

RESP=$(run_sql "$QUERY") || {
    echo "$TAG: schema query failed (non-fatal), skipping"
    exit 0
}

SCHEMAS=$(echo "$RESP" | jq -r '.data[]?[0]? // empty' 2>/dev/null)

if [[ -z "$SCHEMAS" ]]; then
    echo "$TAG: no orphaned schemas found"
    exit 0
fi

# --- Drop each orphaned schema -----------------------------------------------
COUNT=0
while IFS= read -r schema; do
    [[ -z "$schema" ]] && continue
    if [[ ! "$schema" =~ ^TEMP_TEST_SCHEMA_[0-9]+$ ]]; then
        echo "$TAG: skipping unexpected schema name: ${schema}"
        continue
    fi
    echo "$TAG: dropping ${DATABASE}.${schema}"
    run_sql "DROP SCHEMA IF EXISTS ${DATABASE}.${schema} CASCADE" >/dev/null || true
    COUNT=$((COUNT + 1))
done <<< "$SCHEMAS"

echo "$TAG: dropped ${COUNT} orphaned schema(s)"
