#!/bin/bash
# Compare Universal Driver vs Official Snowflake ODBC Driver
# This script runs the same demo with both drivers and compares results

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DEMO_BIN="$PROJECT_ROOT/odbc_tests/cmake-build/demo/advanced_demo"
PARAMS="$PROJECT_ROOT/parameters.json"
UNIV_RAW="/tmp/universal_output.txt"
OFFICIAL_RAW="/tmp/official_output.txt"
UNIV_SANITIZED="/tmp/universal_output.sanitized.txt"
OFFICIAL_SANITIZED="/tmp/official_output.sanitized.txt"

sanitize_output() {
    local input_file="$1"
    local output_file="$2"
    sed -E \
        -e '/^DEBUG:/d' \
        -e '/^Initializing logging/d' \
        -e 's/DRIVER=[^;]*;/DRIVER=<driver>;/g' \
        -e '/^PERF\|/!s/[0-9]+ms/<ms>/g' \
        "$input_file" > "$output_file"
}

print_perf_summary() {
    local universal_raw="$1"
    local official_raw="$2"
    python3 - "$universal_raw" "$official_raw" <<'PY'
import sys
from pathlib import Path

def parse_perf(path):
    data = {}
    try:
        with open(path, "r", encoding="utf-8", errors="ignore") as fh:
            for line in fh:
                if not line.startswith("PERF|"):
                    continue
                parts = line.strip().split("|", 2)
                if len(parts) != 3:
                    continue
                metric = parts[1]
                if "=" not in parts[2]:
                    continue
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

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  ODBC Driver Comparison Test${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""

# Check if official driver exists
OFFICIAL_DRIVER="/opt/snowflake/snowflakeodbc/lib/universal/libSnowflake.dylib"
if [ ! -f "$OFFICIAL_DRIVER" ]; then
    echo -e "${YELLOW}⚠️  Official Snowflake ODBC driver not found at:${NC}"
    echo "   $OFFICIAL_DRIVER"
    echo ""
    echo "   To install, visit: https://docs.snowflake.com/en/user-guide/odbc-download.html"
    echo ""
    echo -e "${YELLOW}   Skipping comparison test.${NC}"
    exit 0
fi

# Check if demo is built
if [ ! -f "$DEMO_BIN" ]; then
    echo -e "${RED}✗ Demo binary not found. Building...${NC}"
    cd "$PROJECT_ROOT/odbc_tests"
    mkdir -p cmake-build && cd cmake-build
    cmake .. && make advanced_demo
    cd "$PROJECT_ROOT"
fi

# Test 1: Universal Driver
echo -e "${GREEN}Testing Universal Driver...${NC}"
UNIVERSAL_DRIVER="$PROJECT_ROOT/target/release/libsfodbc.dylib"
if [ ! -f "$UNIVERSAL_DRIVER" ]; then
    echo -e "${RED}✗ Universal driver not found. Building...${NC}"
    cd "$PROJECT_ROOT"
    cargo build -p odbc --release
fi

RUST_LOG=${RUST_LOG:-warn} \
DRIVER_PATH="$UNIVERSAL_DRIVER" \
PARAMETER_PATH="$PARAMS" \
"$DEMO_BIN" "$PARAMS" > "$UNIV_RAW" 2>&1 || true

echo -e "${GREEN}✓ Universal driver test complete${NC}"
echo ""

# Test 2: Official Driver
echo -e "${GREEN}Testing Official Snowflake Driver...${NC}"
DYLD_LIBRARY_PATH=/opt/homebrew/lib:$DYLD_LIBRARY_PATH \
DRIVER_PATH="$OFFICIAL_DRIVER" \
PARAMETER_PATH="$PARAMS" \
"$DEMO_BIN" "$PARAMS" > "$OFFICIAL_RAW" 2>&1 || true

echo -e "${GREEN}✓ Official driver test complete${NC}"
echo ""

# Compare results
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  Comparison Results${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""

# Extract key metrics
echo -e "${YELLOW}Universal Driver Results:${NC}"
grep "Results:" "$UNIV_RAW" || echo "  (No summary found)"
echo ""

echo -e "${YELLOW}Official Driver Results:${NC}"
grep "Results:" "$OFFICIAL_RAW" || echo "  (No summary found)"
echo ""

# Check for differences
sanitize_output "$UNIV_RAW" "$UNIV_SANITIZED"
sanitize_output "$OFFICIAL_RAW" "$OFFICIAL_SANITIZED"

if diff -q "$UNIV_SANITIZED" "$OFFICIAL_SANITIZED" > /dev/null 2>&1; then
    echo -e "${GREEN}✓ PERFECT MATCH!${NC}"
    echo "  Both drivers produced identical output."
else
    echo -e "${YELLOW}⚠️  Differences detected${NC}"
    echo ""
    echo "  Running detailed diff..."
    echo ""
    
    # Show diff with context
    diff -u "$UNIV_SANITIZED" "$OFFICIAL_SANITIZED" | head -100 || true
    
    echo ""
    echo -e "${BLUE}Full outputs saved to:${NC}"
    echo "  Universal (raw):   $UNIV_RAW"
    echo "  Official (raw):    $OFFICIAL_RAW"
    echo "  Universal (clean): $UNIV_SANITIZED"
    echo "  Official (clean):  $OFFICIAL_SANITIZED"
    echo ""
    echo "  Run 'diff -u /tmp/official_output.txt /tmp/universal_output.txt' for full comparison"
fi

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  Performance Comparison${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
print_perf_summary "$UNIV_RAW" "$OFFICIAL_RAW"
echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"

