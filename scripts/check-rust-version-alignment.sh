#!/usr/bin/env bash
# Verify every explicit `rust-version` pin in the repo's Cargo.toml files matches
# the pinned toolchain (rust-toolchain.toml channel) at major.minor.
#
# The perf image builds crates in isolation (no root workspace present), so we
# can't use [workspace.package] rust-version inheritance — sf_core / odbc / the
# core perf app pin rust-version explicitly. This check keeps those pins from
# drifting apart or away from the toolchain, which the MSRV-aware resolver in
# the perf Dockerfiles (CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback)
# relies on to avoid selecting dependencies the pinned toolchain can't build.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

channel=$(grep -E '^[[:space:]]*channel[[:space:]]*=' "$root/rust-toolchain.toml" | head -1 | sed -E 's/.*"([^"]+)".*/\1/')
tc_mm=$(printf '%s' "$channel" | grep -oE '^[0-9]+\.[0-9]+' || true)
if [[ -z "$tc_mm" ]]; then
  echo "check-rust-version-alignment: could not parse channel from rust-toolchain.toml ('$channel')" >&2
  exit 1
fi

status=0
while IFS= read -r f; do
  ver=$(grep -E '^[[:space:]]*rust-version[[:space:]]*=[[:space:]]*"' "$root/$f" | head -1 | sed -E 's/.*"([^"]+)".*/\1/' || true)
  [[ -z "$ver" ]] && continue
  mm=$(printf '%s' "$ver" | grep -oE '^[0-9]+\.[0-9]+' || true)
  if [[ "$mm" != "$tc_mm" ]]; then
    echo "rust-version misalignment in $f: rust-version=$ver (=> $mm) vs rust-toolchain.toml channel=$channel (=> $tc_mm)" >&2
    status=1
  fi
done < <(cd "$root" && { git ls-files '*Cargo.toml' 2>/dev/null || find . -name Cargo.toml -not -path '*/target/*'; })

if [[ $status -ne 0 ]]; then
  echo "Update the mismatched rust-version(s) to match rust-toolchain.toml (major.minor)." >&2
  exit 1
fi
echo "rust-version aligned: all Cargo.toml pins match toolchain $channel"
