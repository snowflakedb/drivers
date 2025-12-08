#!/bin/bash

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PARAMS="$PROJECT_ROOT/parameters.json"
PERF_BUILD_DIR="$PROJECT_ROOT/odbc_tests/cmake-build"
PERF_BIN="$PERF_BUILD_DIR/perf/perf_runner"

UNIV_OUTPUT="/tmp/perf_universal_output.txt"
OFF_OUTPUT="/tmp/perf_official_output.txt"

print_perf_summary() {
    local universal_raw="$1"
    local official_raw="$2"
    python3 - "$universal_raw" "$official_raw" <<'PY'
import sys

def parse_perf(path):
    data = {}
    try:
        with open(path, "r", encoding="utf-8", errors="ignore") as fh:
            for line in fh:
                if not line.startswith("PERF|"):
                    continue
                parts = line.strip().split("|", 2)
                if len(parts) != 3 or "=" not in parts[2]:
                    continue
                metric = parts[1]
                key, value = parts[2].split("=", 1)
                try:
                    num = float(value)
                except ValueError:
                    continue
                data.setdefault(metric, {})[key] = num
    except FileNotFoundError:
        pass
    return data

univ = parse_perf(sys.argv[1])
off = parse_perf(sys.argv[2])
metrics = sorted(set(univ.keys()) | set(off.keys()))

def fmt_val(val):
    if val is None:
        return "-"
    if abs(val - round(val)) < 1e-9:
        return f"{int(round(val))}"
    if abs(val) >= 1000:
        return f"{val:,.1f}"
    return f"{val:.2f}"

if not metrics:
    print("  (No PERF markers found)")
    sys.exit(0)

header = ["Metric / Key", "Universal", "Official", "Delta", "Delta %"]
fmt = "{:<30} {:>12} {:>12} {:>12} {:>10}"
print(fmt.format(*header))
print("-" * 84)

for metric in metrics:
    keys = sorted(set(univ.get(metric, {}).keys()) | set(off.get(metric, {}).keys()))
    for key in keys:
        u = univ.get(metric, {}).get(key)
        o = off.get(metric, {}).get(key)
        delta = None
        pct = None
        if u is not None and o is not None:
            delta = u - o
            if o != 0:
                pct = (delta / o) * 100.0
        metric_key = f"{metric}:{key}"
        delta_str = fmt_val(delta)
        pct_str = "-" if pct is None else f"{pct:+.2f}%"
        print(fmt.format(metric_key[:30], fmt_val(u), fmt_val(o), delta_str, pct_str))
PY
}

echo "═══════════════════════════════════════════════════════════════"
echo "  ODBC Driver Performance Harness"
echo "═══════════════════════════════════════════════════════════════"

if [ ! -f "$PARAMS" ]; then
    echo "parameters.json not found at $PARAMS"
    exit 1
fi

if [ ! -d "$PERF_BUILD_DIR" ]; then
    mkdir -p "$PERF_BUILD_DIR"
fi

pushd "$PERF_BUILD_DIR" >/dev/null
cmake .. >/tmp/perf_cmake.out 2>&1 && tail -n 10 /tmp/perf_cmake.out
cmake --build . --target perf_runner
popd >/dev/null

OFFICIAL_DRIVER="/opt/snowflake/snowflakeodbc/lib/universal/libSnowflake.dylib"
if [ ! -f "$OFFICIAL_DRIVER" ]; then
    echo "Official Snowflake driver not found at $OFFICIAL_DRIVER"
    exit 1
fi

UNIVERSAL_DRIVER="$PROJECT_ROOT/target/release/libsfodbc.dylib"
if [ ! -f "$UNIVERSAL_DRIVER" ]; then
    echo "Building universal driver..."
    cargo build -p odbc --release
fi

ITERATIONS=${PERF_ITERATIONS:-5}

echo "Running universal driver perf harness..."
RUST_LOG=${RUST_LOG:-warn} \
DRIVER_PATH="$UNIVERSAL_DRIVER" \
PARAMETER_PATH="$PARAMS" \
"$PERF_BIN" --params "$PARAMS" --iterations "$ITERATIONS" > "$UNIV_OUTPUT"

echo "Running official driver perf harness..."
DYLD_LIBRARY_PATH=/opt/homebrew/opt/libiodbc/lib:/opt/homebrew/lib:${DYLD_LIBRARY_PATH:-} \
DRIVER_PATH="$OFFICIAL_DRIVER" \
PARAMETER_PATH="$PARAMS" \
"$PERF_BIN" --params "$PARAMS" --iterations "$ITERATIONS" > "$OFF_OUTPUT"

echo ""
echo "Universal perf output: $UNIV_OUTPUT"
echo "Official perf output:  $OFF_OUTPUT"
echo ""

echo "═══════════════════════════════════════════════════════════════"
echo "  Aggregated Results"
echo "═══════════════════════════════════════════════════════════════"
print_perf_summary "$UNIV_OUTPUT" "$OFF_OUTPUT"
echo "═══════════════════════════════════════════════════════════════"

