#!/usr/bin/env bash
# Restore, regenerate proto, build the .NET solution and native libraries, then run one xUnit v3 test project.
set -euo pipefail

# shellcheck source=scripts/lib/test-common.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib/test-common.sh"

usage() {
    cat <<'EOF'
Usage: scripts/test-dotnet.sh <suite> [--framework TFM] [--skip-build] [-- xunit args]

Suites:
  interop        Snowflake.Data.Tests.Interop (sf_core_stub)            no credentials
  unit           Snowflake.Data.Tests.Unit                              no credentials
  e2e            Snowflake.Data.Tests.E2E on sf_core                    needs parameters.json
  e2e-old        Snowflake.Data.Tests.E2E on old Snowflake.Data         needs parameters.json
  wiremock       Snowflake.Data.Tests.Wiremock on sf_core               needs parameters.json, java
  wiremock-old   Snowflake.Data.Tests.Wiremock on old Snowflake.Data    needs parameters.json, java

Options:
  --framework TFM   net8.0, net9.0, net10.0 or net11.0 (default net10.0)
  --skip-build      Skip restore, proto regeneration and build; reuse the last build

xUnit args go to the in-process runner (it does not accept --filter), e.g.
  -class "*ConnectionTest"   -method "*Connects*"   -list tests

Examples:
  scripts/test-dotnet.sh unit
  scripts/test-dotnet.sh e2e -- -class "*ConnectionTest"
EOF
}

SUITE=""
FRAMEWORK=net10.0
SKIP_BUILD=0
XUNIT_ARGS=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        -h | --help) usage; exit 0 ;;
        --framework) FRAMEWORK="${2:?--framework needs a value}"; shift ;;
        --skip-build) SKIP_BUILD=1 ;;
        --) shift; XUNIT_ARGS=("$@"); break ;;
        -*) die "unknown option $1" ;;
        *) [[ -z "$SUITE" ]] || die "unexpected argument $1"; SUITE="$1" ;;
    esac
    shift
done
check_suite usage "$SUITE" interop unit e2e e2e-old wiremock wiremock-old

case "$FRAMEWORK" in
    net472 | net481) die "$FRAMEWORK builds only on Windows; use net8.0, net9.0, net10.0 or net11.0" ;;
esac

require_tool dotnet "install the .NET SDK from https://dot.net"
ensure_cargo
# ITFixture loads sf_core from <repo>/target/debug and StubFixture loads the stub from
# sf_core_stub/target/debug, so cargo must build into the default target directories.
unset CARGO_TARGET_DIR

NEEDS_CORE=0
case "$SUITE" in
    interop) PROJECT=Snowflake.Data.Tests.Interop ;;
    unit) PROJECT=Snowflake.Data.Tests.Unit; NEEDS_CORE=1 ;;
    e2e) PROJECT=Snowflake.Data.Tests.E2E; NEEDS_CORE=1 ;;
    e2e-old) PROJECT=Snowflake.Data.Tests.E2E; export SNOWFLAKE_DOTNET_USE_OLD_DRIVER=1 ;;
    wiremock) PROJECT=Snowflake.Data.Tests.Wiremock; NEEDS_CORE=1 ;;
    wiremock-old) PROJECT=Snowflake.Data.Tests.Wiremock; export SNOWFLAKE_DOTNET_USE_OLD_DRIVER=1 ;;
esac

case "$SUITE" in
    e2e | e2e-old | wiremock | wiremock-old) require_parameters ;;
esac
case "$SUITE" in
    wiremock | wiremock-old) require_tool java "the Wiremock tests run the WireMock standalone JAR with java" ;;
esac

cd "$REPO_ROOT"

if [[ "$NEEDS_CORE" == 1 ]]; then
    run cargo build --locked --package sf_core
    SF_CORE_LIB_PATH="$REPO_ROOT/target/debug/$(native_lib_name sf_core)"
    export SF_CORE_LIB_PATH
    [[ -f "$SF_CORE_LIB_PATH" ]] || die "expected $SF_CORE_LIB_PATH after cargo build"
fi

if [[ "$SKIP_BUILD" == 0 ]]; then
    cd "$REPO_ROOT/dotnet"
    run dotnet restore
    run dotnet msbuild -t:RegenerateProto src/Snowflake.Data.Proto/
    skip_core=true
    [[ "$NEEDS_CORE" == 1 ]] && skip_core=false
    run dotnet build --no-restore "-p:SkipSfCoreBuild=$skip_core"
fi

cd "$REPO_ROOT/dotnet/tests/$PROJECT"
if [[ ${#XUNIT_ARGS[@]} -gt 0 ]]; then
    run dotnet run --no-build --framework "$FRAMEWORK" -- "${XUNIT_ARGS[@]}"
else
    run dotnet run --no-build --framework "$FRAMEWORK"
fi
