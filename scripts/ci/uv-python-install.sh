#!/usr/bin/env bash
# Retry `uv python install` when GitHub's python-build-standalone release
# asset returns HTTP 500 (SNOW-4241880). uv's own HTTP retries hit the same
# 500 back-to-back; space the outer attempts so a short release-asset blip
# can clear. CI-only. Does not change which CPython build is installed.
set -euo pipefail

request="${1:?usage: uv-python-install.sh <uv-python-request>}"
max=6
for attempt in $(seq 1 "$max"); do
  if uv python install "$request"; then
    exit 0
  fi
  if [ "$attempt" -eq "$max" ]; then
    echo "::error::uv python install exhausted ${max} attempts: ${request}" >&2
    exit 1
  fi
  echo "uv python install failed (attempt ${attempt}/${max}): ${request}; retrying"
  sleep $((15 * attempt))
done
