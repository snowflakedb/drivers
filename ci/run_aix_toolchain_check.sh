#!/usr/bin/env bash
# SSH to the AIX host and run toolchain health checks (no cargo build).
set -euo pipefail

: "${AIX_SSH_HOST:?AIX_SSH_HOST is required}"
: "${AIX_SSH_USER:?AIX_SSH_USER is required}"
: "${AIX_SSH_PRIVATE_KEY_B64:?AIX_SSH_PRIVATE_KEY_B64 is required}"
: "${AIX_SSH_KNOWN_HOSTS_B64:?AIX_SSH_KNOWN_HOSTS_B64 is required}"

KEYFILE="$(mktemp)"
KNOWN_HOSTS="$(mktemp)"
cleanup() {
  rm -f "$KEYFILE" "$KNOWN_HOSTS"
}
trap cleanup EXIT

umask 077
printf '%s' "$AIX_SSH_PRIVATE_KEY_B64" | base64 -d >"$KEYFILE"
printf '%s' "$AIX_SSH_KNOWN_HOSTS_B64" | base64 -d >"$KNOWN_HOSTS"
chmod 600 "$KEYFILE" "$KNOWN_HOSTS"

SSH_OPTS=(
  -i "$KEYFILE"
  -o IdentitiesOnly=yes
  -o BatchMode=yes
  -o StrictHostKeyChecking=yes
  -o UserKnownHostsFile="$KNOWN_HOSTS"
  -o ConnectTimeout=30
  -o ServerAliveInterval=15
  -o ServerAliveCountMax=4
)
SSH=(ssh "${SSH_OPTS[@]}" "${AIX_SSH_USER}@${AIX_SSH_HOST}")

echo "--- :aix: AIX toolchain check"
"${SSH[@]}" "bash -s" <<'EOF'
set -euo pipefail
export PATH="/opt/freeware/bin:$HOME/.cargo/bin:$PATH"
fail=0
probe_dir="$(mktemp -d)"
trap 'rm -rf "$probe_dir"' EXIT
echo "=== uname ==="
uname -a
echo "=== rustc ==="
if ! command -v rustc >/dev/null 2>&1; then
  echo "rustc is not on PATH" >&2
  fail=1
else
  rustc -vV
  rustc -vV | grep -q 'host: powerpc64-ibm-aix' || {
    echo "rustc host is not powerpc64-ibm-aix" >&2
    fail=1
  }
fi
echo "=== gcc -maix64 ==="
if ! command -v gcc >/dev/null 2>&1; then
  echo "gcc is not on PATH" >&2
  fail=1
else
  gcc -dumpversion
  cat >"$probe_dir/probe.c" <<'C'
#include <stdio.h>
int main(void) {
  printf("pointer size: %d\n", (int)sizeof(void *));
  return sizeof(void *) == 8 ? 0 : 1;
}
C
  if ! gcc -maix64 -o "$probe_dir/probe" "$probe_dir/probe.c"; then
    echo "gcc -maix64 failed to compile and link the probe" >&2
    fail=1
  elif ! file "$probe_dir/probe" | grep -q '64-bit XCOFF'; then
    echo "probe is not a 64-bit XCOFF executable:" >&2
    file "$probe_dir/probe" >&2
    fail=1
  elif ! "$probe_dir/probe"; then
    echo "probe failed to run as a 64-bit executable" >&2
    fail=1
  fi
fi
echo "=== g++ ==="
command -v g++ >/dev/null 2>&1 && g++ -maix64 -dumpversion || echo "g++ not on PATH (warning)"
echo "=== protoc (informational until M4) ==="
if command -v protoc >/dev/null 2>&1; then
  command -v protoc
  protoc --version || true
else
  echo "protoc not on PATH"
fi
echo "=== PATH ==="
echo "$PATH"
exit $fail
EOF
