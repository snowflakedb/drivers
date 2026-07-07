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

# The runtime image is minimal; the vendored-openssl binary only needs glibc +
# libstdc++, and rustls-native-certs needs the system CA bundle. Install the
# few runtime bits defensively (no-op if already present).
dnf install -y --setopt=install_weak_deps=False libstdc++ ca-certificates >/dev/null 2>&1 || true

export PARAMETER_PATH="$(pwd)/parameters.json"

chmod +x sf_core_e2e
./sf_core_e2e authentication::workload_identity --nocapture --test-threads=1
