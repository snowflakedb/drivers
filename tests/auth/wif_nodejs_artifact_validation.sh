#!/bin/bash
#
# Validates a reused nodejs_wif.tar.gz before `run_wif_local.sh nodejs
# --skip-build` ships it to the WIF VMs.
#
# The VMs unpack and run the tarball, not the nodejs/ tree of this checkout.
# After a branch switch, a new commit, or a move to another worktree (common
# when agents drive the run), --skip-build would otherwise test code other
# than what is checked out, with nothing in the log to show it.
#
# A full build writes nodejs_wif.tar.gz.sidecar, a key=value file recording the
# tarball's sha256 and the git commit, dirty state, source hash, Rust channel,
# builder image, and platform that produced it. skip_build_or_die compares that
# record with the current checkout:
#   * A missing tarball or a sha256 mismatch is an error: the sidecar no longer
#     describes the bytes that would be shipped.
#   * Any other difference is a warning. Reusing an older tarball is what
#     --skip-build is for, so the run continues and the log names the commit
#     whose code the VMs run.
# A tarball without a sidecar is reused with a warning.
#
# Sourced by tests/auth/run_wif_local.sh. Jenkins builds the tarball on every
# run and does not use it.

file_sha256() {
  local path="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$path" | awk '{print $1}'
  else
    shasum -a 256 "$path" | awk '{print $1}'
  fi
}

# stat and date name these lookups differently on GNU and BSD. Each rejects the
# other's flag without writing to stdout, so the fallback captures the value
# alone.
file_mtime_utc() {
  local path="$1" epoch
  epoch="$(stat -c %Y "$path" 2>/dev/null || stat -f %m "$path")"
  date -u -d "@$epoch" +'%Y-%m-%dT%H:%M:%SZ' 2>/dev/null \
    || date -u -r "$epoch" +'%Y-%m-%dT%H:%M:%SZ'
}

rust_channel() {
  awk -F'"' '/^channel[[:space:]]*=/{print $2; exit}' "$REPO_ROOT/rust-toolchain.toml"
}

source_fingerprint() {
  {
    git -C "$REPO_ROOT" rev-parse HEAD
    git -C "$REPO_ROOT" status --porcelain
  } | file_sha256 /dev/stdin
}

git_dirty() {
  if git -C "$REPO_ROOT" diff-index --quiet HEAD -- \
    && [[ -z "$(git -C "$REPO_ROOT" ls-files --others --exclude-standard)" ]]; then
    echo false
  else
    echo true
  fi
}

sidecar_path() {
  echo "${ARTIFACT}.sidecar"
}

write_nodejs_sidecar() {
  local image_id="$1" platform="$2"
  local sha bytes mtime rust git_commit dirty source_hash
  sha="$(file_sha256 "$ARTIFACT")"
  bytes="$(wc -c < "$ARTIFACT" | tr -d ' ')"
  mtime="$(file_mtime_utc "$ARTIFACT")"
  rust="$(rust_channel)"
  git_commit="$(git -C "$REPO_ROOT" rev-parse HEAD)"
  dirty="$(git_dirty)"
  source_hash="$(source_fingerprint)"

  cat > "$(sidecar_path)" <<EOF
sha256=${sha}
bytes=${bytes}
mtime=${mtime}
git=${git_commit}
dirty=${dirty}
source-hash=${source_hash}
platform=${platform}
rust=${rust}
image=${image_id}
EOF
}

sidecar_get() {
  local key="$1" file="$2"
  awk -F= -v k="$key" '$1==k {print substr($0, index($0, "=")+1); exit}' "$file"
}

warn_if_differ() {
  local label="$1" recorded="$2" current="$3"
  if [[ "$recorded" != "$current" ]]; then
    echo "WARN: $label differs from this checkout; reusing the tarball anyway." >&2
    echo "  sidecar: $recorded" >&2
    echo "  now:     $current" >&2
  fi
}

skip_build_or_die() {
  local sidecar recorded_sha actual_sha packed_git now_git
  if [[ ! -f "$ARTIFACT" ]]; then
    echo "ERROR: $ARTIFACT not found. Run without --skip-build first." >&2
    exit 1
  fi

  echo "Reusing $ARTIFACT"
  echo "  bytes=$(wc -c < "$ARTIFACT" | tr -d ' ')"

  sidecar="$(sidecar_path)"
  if [[ ! -f "$sidecar" ]]; then
    echo "WARN: no sidecar at $sidecar; cannot tell which commit or image produced this tarball." >&2
    return 0
  fi

  cat "$sidecar"
  recorded_sha="$(sidecar_get sha256 "$sidecar")"
  actual_sha="$(file_sha256 "$ARTIFACT")"
  if [[ -n "$recorded_sha" && "$actual_sha" != "$recorded_sha" ]]; then
    echo "ERROR: $ARTIFACT does not match the sidecar hash." >&2
    echo "  sidecar: $recorded_sha" >&2
    echo "  file:    $actual_sha" >&2
    exit 1
  fi

  packed_git="$(sidecar_get git "$sidecar")"
  now_git="$(git -C "$REPO_ROOT" rev-parse HEAD)"
  echo "Node WIF tests will use code from sha: ${packed_git:-unknown} from path $ARTIFACT"
  if [[ -n "$packed_git" && "$packed_git" != "$now_git" ]]; then
    echo "WARN: this checkout is $now_git; the VM unpacks that tarball, not nodejs/ from this SHA." >&2
  fi
  warn_if_differ "dirty" "$(sidecar_get dirty "$sidecar")" "$(git_dirty)"
  warn_if_differ "source-hash" "$(sidecar_get source-hash "$sidecar")" "$(source_fingerprint)"
  warn_if_differ "rust" "$(sidecar_get rust "$sidecar")" "$(rust_channel)"
  warn_if_differ "image" "$(sidecar_get image "$sidecar")" "$IMAGE_ID"
  warn_if_differ "platform" "$(sidecar_get platform "$sidecar")" "linux/amd64"
}
