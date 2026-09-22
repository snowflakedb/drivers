#!/bin/bash -e
#
# Runs INSIDE the public runtime container on a WIF cloud VM. The outer
# ci/test_wif.sh scp's the prebuilt artifacts plus this script and the
# generated parameters.json into /tests, then `docker run`s a public image
# (rockylinux:8 — the same base as the coverage image used to build) with
# /tests mounted and invokes this script.
#
# The cloud identity (AWS role / Azure MI / GCP SA) is exposed to the container
# via the VM's IMDS, so the WIF e2e tests authenticate as that identity.
#
# Expected layout in the working directory (/tests):
#   sf_core_e2e       prebuilt e2e_tests binary
#   parameters.json   test parameters (PARAMETER_PATH points here)

set -o pipefail

cd "$(dirname "${BASH_SOURCE[0]}")"

chmod +x sf_core_e2e

# The runtime image is minimal. The e2e binary needs glibc + libstdc++, plus
# openssl-libs: sf_core keeps `openssl` as a dev-dependency for test fixtures
# and links it dynamically, even though the driver itself no longer uses
# OpenSSL. rustls-native-certs needs the system CA bundle.
#
# This install is load-bearing rather than defensive, which is why it is not
# suppressed and not `|| true`-ed. Since the driver stopped linking OpenSSL,
# the test binary is the only thing left pulling in libssl, so a failed
# install no longer degrades gracefully: the run dies later with a
# missing-library error that reads like a WIF authentication failure. Let
# dnf's own diagnostics reach the log and let `-e` stop the script.
dnf install -y --setopt=install_weak_deps=False libstdc++ ca-certificates openssl-libs

# A clean dnf exit does not prove *this* binary resolves, so check the end
# state directly. Assigning first rather than piping into grep matters: inside
# an `if` condition `-e` is suspended, so a failing `ldd` would otherwise look
# like "no unresolved libraries" and pass.
ldd_out=$(ldd sf_core_e2e)
if grep -q 'not found' <<<"$ldd_out"; then
    echo "ERROR: sf_core_e2e has unresolved shared libraries:" >&2
    grep 'not found' <<<"$ldd_out" >&2
    exit 1
fi

export PARAMETER_PATH="$(pwd)/parameters.json"

./sf_core_e2e authentication::workload_identity --nocapture --test-threads=1
