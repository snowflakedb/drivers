#!/usr/bin/env bash
# Run ./gradlew, retrying transient CI infra that aborts before any test task.
#
# GitHub-hosted runners hit Maven Central 403/429/5xx during configuration, and
# Windows Java 21 also flakes downloading the Gradle distribution from
# services.gradle.org (SSLHandshakeException: Remote host terminated the
# handshake in org.gradle.wrapper.Install). Jenkins sidesteps Maven Central
# through Artifactory, but that proxy is on a private network and is
# unreachable from hosted runners, so retry instead.
#
# Genuine test failures are returned on the first attempt: if a test task
# already started, SSL/HTTP noise in the log is not treated as infra.
set -uo pipefail

MAX_ATTEMPTS="${GRADLE_RESOLVE_RETRY_ATTEMPTS:-5}"
BACKOFF_SECONDS="${GRADLE_RESOLVE_RETRY_BACKOFF_SECONDS:-15}"

# Markers Gradle/JVM print for repository/plugin resolution failures and
# wrapper TLS aborts. Deliberately phrased as they emit them so test output
# that merely mentions 403 (e.g. ExternalBrowserTests) cannot trigger a retry.
TRANSIENT_INFRA_PATTERN='Could not resolve all (artifacts|dependencies|files) for configuration|Could not GET .https?://|Received status code (403|4[0-9]{2}|5[0-9]{2}) from server|was not found in any of the following sources|SSLHandshakeException|Remote host terminated the handshake|SSL peer shut down incorrectly|org\.gradle\.wrapper\.Install'

# gradle task names used with this wrapper in .github/workflows/test-jdbc.yml
TEST_STARTED_PATTERN='> Task :(test|referenceTest|parityTest)|Gradle Test Executor'

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

  if grep -Eq "$TEST_STARTED_PATTERN" "$log_file"; then
    echo "gradlew_with_resolve_retry: tests already started; not retrying." >&2
    exit "$status"
  fi

  if ! grep -Eq "$TRANSIENT_INFRA_PATTERN" "$log_file"; then
    echo "gradlew_with_resolve_retry: failure is not a Gradle infra error; not retrying." >&2
    exit "$status"
  fi

  sleep_seconds=$(( BACKOFF_SECONDS * attempt ))
  echo "gradlew_with_resolve_retry: transient Gradle infra failure; retrying in ${sleep_seconds}s." >&2
  sleep "$sleep_seconds"
done
