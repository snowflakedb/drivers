#!/usr/bin/env bash
# Upload JUnit XML results to Buildkite Artifacts + Test Analytics,
# then annotate the build with pass/fail status.
#
# Usage: upload_test_results.sh <driver> <label> <docker_exit> <file_pattern> [file_pattern...]
#
# Examples:
#   ci/upload_test_results.sh rust  "Rust Core" "$DOCKER_EXIT" "junit-results/rust-junit.xml"
#   ci/upload_test_results.sh jdbc  "JDBC"      "$DOCKER_EXIT" "junit-results/TEST-*.xml"

DRIVER="$1"
LABEL="$2"
DOCKER_EXIT="$3"
shift 3

RESPONSE_FILE=$(mktemp "${TMPDIR:-/tmp}/analytics-response.XXXXXX")
trap 'rm -f "$RESPONSE_FILE"' EXIT

echo "--- :buildkite: Uploading test results"

for pattern in "$@"; do
  buildkite-agent artifact upload "$pattern" || true
done

UPLOAD_FAILURES=0

for pattern in "$@"; do
  for xml in $pattern; do
    [ -f "$xml" ] || { echo "WARNING: JUnit XML not found: $xml"; continue; }
    echo "Uploading $xml to Test Analytics (driver=$DRIVER)..."
    HTTP_CODE=$(curl -s -S -X POST --max-time 30 \
      -w "%{http_code}" -o "$RESPONSE_FILE" \
      -H "Authorization: Token token=$BUILDKITE_ANALYTICS_TOKEN" \
      -F "data=@$xml" \
      -F "format=junit" \
      -F "tags[driver]=$DRIVER" \
      -F "run_env[CI]=buildkite" \
      -F "run_env[key]=$BUILDKITE_BUILD_ID" \
      -F "run_env[number]=$BUILDKITE_BUILD_NUMBER" \
      -F "run_env[job_id]=$BUILDKITE_JOB_ID" \
      -F "run_env[branch]=$BUILDKITE_BRANCH" \
      -F "run_env[commit_sha]=$BUILDKITE_COMMIT" \
      -F "run_env[message]=$BUILDKITE_MESSAGE" \
      -F "run_env[url]=$BUILDKITE_BUILD_URL" \
      https://analytics-api.buildkite.com/v1/uploads) || true

    if [ -n "$HTTP_CODE" ] && [ "$HTTP_CODE" -ge 200 ] 2>/dev/null && [ "$HTTP_CODE" -lt 300 ] 2>/dev/null; then
      echo "Uploaded $xml to Test Analytics (HTTP $HTTP_CODE)"
    else
      echo "WARNING: Test Analytics upload failed for $xml (HTTP ${HTTP_CODE:-unknown})"
      cat "$RESPONSE_FILE" 2>/dev/null || true
      echo ""
      UPLOAD_FAILURES=$((UPLOAD_FAILURES + 1))
    fi
  done
done

if [ "$UPLOAD_FAILURES" -gt 0 ]; then
  echo "WARNING: $UPLOAD_FAILURES Test Analytics upload(s) failed"
  buildkite-agent annotate ":warning: $LABEL -- $UPLOAD_FAILURES Test Analytics upload(s) failed" --style "warning" --context "${DRIVER}-upload" || true
fi

if [ "$DOCKER_EXIT" -ne 0 ]; then
  buildkite-agent annotate ":x: $LABEL -- tests failed" --style "error" --context "${DRIVER}-result" || true
  exit "$DOCKER_EXIT"
fi
buildkite-agent annotate ":white_check_mark: $LABEL -- passed" --style "success" --context "${DRIVER}-result" || true
