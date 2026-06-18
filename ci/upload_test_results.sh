#!/usr/bin/env bash
# Upload JUnit XML results to Buildkite Artifacts + Test Analytics,
# then annotate the build with pass/fail status.
#
# Usage: upload_test_results.sh <driver> <label> <docker_exit> <file_pattern> [file_pattern...]
#
# Examples:
#   ci/upload_test_results.sh rust  "Rust Core" "$DOCKER_EXIT" "junit-results/rust-junit.xml"
#   ci/upload_test_results.sh jdbc  "JDBC"      "$DOCKER_EXIT" "junit-results/TEST-*.xml"
set -uo pipefail

DRIVER="$1"
LABEL="$2"
DOCKER_EXIT="$3"
shift 3

echo "--- :buildkite: Uploading test results"

for pattern in "$@"; do
  buildkite-agent artifact upload "$pattern" || true
done

for pattern in "$@"; do
  for xml in $pattern; do
    [ -f "$xml" ] || continue
    curl -X POST --silent --show-error --max-time 30 \
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
      https://analytics-api.buildkite.com/v1/uploads || echo "WARNING: Test Analytics upload failed (non-fatal)"
    echo "Uploaded $xml to Test Analytics"
  done
done

if [ "$DOCKER_EXIT" -ne 0 ]; then
  buildkite-agent annotate ":x: $LABEL -- tests failed" --style "error" --context "${DRIVER}-result"
  exit "$DOCKER_EXIT"
fi
buildkite-agent annotate ":white_check_mark: $LABEL -- passed" --style "success" --context "${DRIVER}-result"
