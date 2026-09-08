#!/usr/bin/env bash

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT/python"

env \
  -u UV_INDEX \
  -u UV_DEFAULT_INDEX \
  -u UV_INDEX_URL \
  -u UV_EXTRA_INDEX_URL \
  -u UV_FIND_LINKS \
  -u UV_CONFIG_FILE \
  -u UV_NO_CONFIG \
  uv lock --config-file uv.toml

unexpected_sources="$(
  grep '^source = { registry = ' uv.lock \
    | grep -v '^source = { registry = "https://pypi.org/simple" }$' \
    || true
)"
if [[ -n "$unexpected_sources" ]]; then
  echo "python/uv.lock contains package sources outside PyPI:" >&2
  echo "$unexpected_sources" >&2
  exit 1
fi
