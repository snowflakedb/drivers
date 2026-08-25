#!/usr/bin/env bash
# Keep python/Cargo.lock.sdist in sync with the reduced sdist Cargo workspace.
#
# hatch_build.py runs `cargo run --locked` / `cargo build --locked` when an
# sdist is installed. If Cargo.lock.sdist is missing a feature-graph edge that
# the sdist manifests require (for example tokio-util gaining futures-util
# after the `rt` feature was enabled), Cargo refuses to proceed.
#
# Usage:
#   scripts/python-sdist-cargo-lock.sh          # fail if --locked would rewrite
#   scripts/python-sdist-cargo-lock.sh refresh  # rewrite lock without version bumps
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mode="${1:-check}"

members=(
  sf_core
  python_bridge
  sf_params_spec
  proto_utils
  error_trace
  error_trace_derive
  proto_generator
)

tmp="$(mktemp -d "${TMPDIR:-/tmp}/python-sdist-cargo-lock.XXXX")"
cleanup() { rm -rf "$tmp"; }
trap cleanup EXIT

cp "$root/python/Cargo.toml.sdist" "$tmp/Cargo.toml"
cp "$root/python/Cargo.lock.sdist" "$tmp/Cargo.lock"
for member in "${members[@]}"; do
  ln -s "$root/$member" "$tmp/$member"
done

cargo_metadata() {
  local extra=()
  if [[ "$1" == "locked" ]]; then
    extra+=(--locked)
  fi
  cargo metadata \
    --manifest-path "$tmp/Cargo.toml" \
    --format-version 1 \
    "${extra[@]}" \
    >/dev/null
}

case "$mode" in
  check)
    if ! cargo_metadata locked; then
      echo "python/Cargo.lock.sdist is stale for the sdist workspace." >&2
      echo "Refresh it with: scripts/python-sdist-cargo-lock.sh refresh" >&2
      exit 1
    fi
    echo "python/Cargo.lock.sdist accepts cargo metadata --locked"
    ;;
  refresh)
    cargo_metadata rewrite
    cp "$tmp/Cargo.lock" "$root/python/Cargo.lock.sdist"
    cargo_metadata locked
    echo "Wrote $root/python/Cargo.lock.sdist"
    ;;
  *)
    echo "Usage: $0 [check|refresh]" >&2
    exit 2
    ;;
esac
