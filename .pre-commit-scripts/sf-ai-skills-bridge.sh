#!/usr/bin/env bash
set -e

if ! command -v sf >/dev/null 2>&1; then
    if [[ -n "${CI:-}" || -n "${GITHUB_ACTIONS:-}" || -n "${JENKINS_URL:-}" || -n "${BUILDKITE:-}" ]]; then
        echo "sf-cli not on PATH; skipping sf ai skills bridge in CI." >&2
        exit 0
    fi
    echo "sf-cli is required but was not found on PATH." >&2
    echo "Install the sf CLI — ask your team lead for install instructions." >&2
    exit 1
fi

if [[ "${SF_AI_SKILLS_BRIDGE_LENIENT:-}" = "1" ]]; then
    sf ai skills bridge --lenient
else
    sf ai skills bridge
fi
