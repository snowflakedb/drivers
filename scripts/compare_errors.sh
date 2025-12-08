#!/bin/bash
# Compare ODBC diagnostic output between universal and official drivers

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
UNIXODBC_HARNESS="$PROJECT_ROOT/odbc_tests/cmake-build/demo/error_harness"
IODBC_HARNESS="$PROJECT_ROOT/odbc_tests/cmake-build-iodbc/demo/error_harness"
PARAMS="$PROJECT_ROOT/parameters.json"
UNIV_RAW="/tmp/error_universal_output.txt"
OFFICIAL_RAW="/tmp/error_official_output.txt"

OFFICIAL_DRIVER="/opt/snowflake/snowflakeodbc/lib/universal/libSnowflake.dylib"
UNIVERSAL_DRIVER="$PROJECT_ROOT/target/release/libsfodbc.dylib"

if [ ! -f "$OFFICIAL_DRIVER" ]; then
    echo "Official Snowflake driver not found at $OFFICIAL_DRIVER"
    exit 1
fi

if [ ! -f "$UNIVERSAL_DRIVER" ]; then
    echo "Building universal driver..."
    cargo build -p odbc --release
fi

if [ ! -f "$UNIXODBC_HARNESS" ]; then
    echo "Building unixODBC harness for universal driver..."
    cd "$PROJECT_ROOT/odbc_tests"
    mkdir -p cmake-build && cd cmake-build
    cmake .. >/dev/null
    make error_harness >/dev/null
fi

if [ ! -f "$IODBC_HARNESS" ]; then
    echo "Building iODBC harness for official driver..."
    cd "$PROJECT_ROOT/odbc_tests"
    mkdir -p cmake-build-iodbc && cd cmake-build-iodbc
    USE_IODBC=1 cmake .. >/dev/null
    make error_harness >/dev/null
fi

echo "Running harness with universal driver (unixODBC)..."
RUST_LOG=${RUST_LOG:-warn} \
DRIVER_PATH="$UNIVERSAL_DRIVER" \
"$UNIXODBC_HARNESS" "$PARAMS" > "$UNIV_RAW" 2>&1 || true

echo "Running harness with official driver (iODBC)..."
DYLD_LIBRARY_PATH=/opt/homebrew/opt/libiodbc/lib:${DYLD_LIBRARY_PATH:-} \
DRIVER_PATH="$OFFICIAL_DRIVER" \
"$IODBC_HARNESS" "$PARAMS" > "$OFFICIAL_RAW" 2>&1 || true

python3 - "$UNIV_RAW" "$OFFICIAL_RAW" <<'PY'
import sys
from collections import defaultdict

def parse_file(path):
    states = defaultdict(list)
    try:
        with open(path, "r", encoding="utf-8", errors="ignore") as fh:
            for line in fh:
                if line.startswith("DIAG|"):
                    parts = line.strip().split("|")[1:]
                    data = {}
                    for part in parts:
                        if "=" in part:
                            key, value = part.split("=", 1)
                            data[key] = value
                    scenario = data.get("scenario", "unknown")
                    states[scenario].append({
                        "state": data.get("state", ""),
                        "native": data.get("native", ""),
                        "message": data.get("message", ""),
                        "rec": data.get("rec", "")
                    })
    except FileNotFoundError:
        pass
    return states

univ = parse_file(sys.argv[1])
official = parse_file(sys.argv[2])
scenarios = sorted(set(univ.keys()) | set(official.keys()))

if not scenarios:
    print("No DIAG lines found in outputs.")
    sys.exit(0)

fmt = "{:<20} {:<10} {:<10} {:<10} {:<10}"
print(fmt.format("Scenario", "Rec", "UnivState", "OffState", "Match?"))
print("-" * 70)
for scenario in scenarios:
    u_records = univ.get(scenario, [])
    o_records = official.get(scenario, [])
    max_len = max(len(u_records), len(o_records), 1)
    for idx in range(max_len):
        u_state = u_records[idx]["state"] if idx < len(u_records) else "-"
        o_state = o_records[idx]["state"] if idx < len(o_records) else "-"
        rec = u_records[idx]["rec"] if idx < len(u_records) else (o_records[idx]["rec"] if idx < len(o_records) else str(idx+1))
        match = "YES" if u_state == o_state else "NO"
        print(fmt.format(scenario[:20], rec, u_state, o_state, match))
PY

echo ""
echo "Universal output: $UNIV_RAW"
echo "Official output : $OFFICIAL_RAW"

