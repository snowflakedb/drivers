#!/usr/bin/env bash
# Run ./gradlew, retrying only when the build fails while resolving dependencies.
#
# Public Maven Central intermittently answers GitHub-hosted runners with 403
# (and occasionally 5xx), which aborts Gradle during configuration before any
# task runs. Jenkins sidesteps this through Artifactory, but that proxy is on a
# private network and is unreachable from hosted runners, so retry instead.
#
# Anything that is not a resolution failure — notably a genuine test failure —
# is returned to the caller on the first attempt.
set -uo pipefail

MAX_ATTEMPTS="${GRADLE_RESOLVE_RETRY_ATTEMPTS:-3}"
BACKOFF_SECONDS="${GRADLE_RESOLVE_RETRY_BACKOFF_SECONDS:-15}"

# Markers Gradle prints for repository/plugin resolution failures. Deliberately
# phrased as Gradle emits them so test output that merely mentions 403 (e.g.
# ExternalBrowserTests) cannot trigger a retry.
RESOLUTION_FAILURE_PATTERN='Could not resolve all (artifacts|dependencies|files) for configuration|Could not GET .https?://|Received status code (403|4[0-9]{2}|5[0-9]{2}) from server|was not found in any of the following sources'

log_file="$(mktemp)"
trap 'rm -f "$log_file"' EXIT

for (( attempt = 1; attempt <= MAX_ATTEMPTS; attempt++ )); do
  echo "gradlew_with_resolve_retry: attempt ${attempt}/${MAX_ATTEMPTS}: ./gradlew $*"

  ./gradlew "$@" 2>&1 | tee "$log_file"
  status="${PIPESTATUS[0]}"

  if [[ "$status" -eq 0 ]]; then
    exit 0
  fi

  if (( attempt == MAX_ATTEMPTS )); then
    echo "gradlew_with_resolve_retry: exhausted ${MAX_ATTEMPTS} attempts; failing with exit code ${status}." >&2
    exit "$status"
  fi

  if ! grep -Eq "$RESOLUTION_FAILURE_PATTERN" "$log_file"; then
    echo "gradlew_with_resolve_retry: failure is not a dependency-resolution error; not retrying." >&2
    exit "$status"
  fi

  sleep_seconds=$(( BACKOFF_SECONDS * attempt ))
  echo "gradlew_with_resolve_retry: transient dependency-resolution failure; retrying in ${sleep_seconds}s." >&2
  sleep "$sleep_seconds"
done
