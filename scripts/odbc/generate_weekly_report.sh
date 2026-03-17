#!/bin/bash

# Generate weekly ODBC implementation progress report.
#
# Produces a Markdown report with:
#   1. Coverage summary from running new tests on the old (reference) driver
#   2. Results of running fully-enabled odbc-api tests on the new driver
#      (SKIP_NEW_DRIVER_NOT_IMPLEMENTED disabled)
#
# Usage:
#   ./scripts/odbc/generate_weekly_report.sh [--skip-reference] [--skip-new-driver]
#
# Options:
#   --skip-reference   Skip old-driver reference coverage (reuse existing coverage_report/summary.txt)
#   --skip-new-driver  Skip new-driver fully-enabled test run (reuse existing results)
#
# Prerequisites:
#   - Docker (for reference/old driver tests)
#   - PARAMETER_PATH set or parameters.json in project root
#   - cargo, cmake, odbc_config in PATH

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
REPORT_DIR="$PROJECT_ROOT/weekly_report"
REPORT_FILE="$REPORT_DIR/report.md"
DATE=$(date +%Y-%m-%d)

SKIP_REFERENCE=false
SKIP_NEW_DRIVER=false

for arg in "$@"; do
    case $arg in
        --skip-reference)  SKIP_REFERENCE=true ;;
        --skip-new-driver) SKIP_NEW_DRIVER=true ;;
        --help|-h)
            head -20 "$0" | tail -18
            exit 0
            ;;
        *)
            echo "Unknown argument: $arg"
            exit 1
            ;;
    esac
done

mkdir -p "$REPORT_DIR"

# ============================================================================
# Helper: parse ctest output for pass/fail/skip summary
# ============================================================================
parse_ctest_results() {
    local junit_file="$1"

    if [ ! -f "$junit_file" ]; then
        echo "  *JUnit file not found: $junit_file*"
        return 1
    fi

    local total tests_attr failures disabled errors skipped passed
    tests_attr=$(grep -o 'tests="[0-9]*"' "$junit_file" | head -1 | grep -o '[0-9]*')
    failures=$(grep -o 'failures="[0-9]*"' "$junit_file" | head -1 | grep -o '[0-9]*')
    disabled=$(grep -o 'disabled="[0-9]*"' "$junit_file" | head -1 | grep -o '[0-9]*')
    errors=$(grep -o 'errors="[0-9]*"' "$junit_file" | head -1 | grep -o '[0-9]*')
    skipped=$(grep -o 'skipped="[0-9]*"' "$junit_file" | head -1 | grep -o '[0-9]*')

    total="${tests_attr:-0}"
    failures="${failures:-0}"
    disabled="${disabled:-0}"
    errors="${errors:-0}"
    skipped="${skipped:-0}"
    passed=$((total - failures - disabled - errors - skipped))

    echo "| Total | Passed | Failed | Skipped | Errors |"
    echo "|-------|--------|--------|---------|--------|"
    echo "| $total | $passed | $failures | $skipped | $errors |"
}

# Extract individual failed test names from JUnit XML
list_failed_tests() {
    local junit_file="$1"
    local max_show="${2:-20}"

    if [ ! -f "$junit_file" ]; then
        return
    fi

    local failed_tests
    failed_tests=$(grep -B1 '<failure' "$junit_file" | grep 'testcase' | sed -E 's/.*name="([^"]+)".*/\1/' || true)

    if [ -z "$failed_tests" ]; then
        return
    fi

    local count
    count=$(echo "$failed_tests" | wc -l | tr -d ' ')

    echo ""
    echo "<details>"
    echo "<summary>Failed tests ($count)</summary>"
    echo ""
    echo '```'
    if [ "$count" -gt "$max_show" ]; then
        echo "$failed_tests" | head -"$max_show"
        echo "... and $((count - max_show)) more"
    else
        echo "$failed_tests"
    fi
    echo '```'
    echo "</details>"
}

# ============================================================================
# Part 1: Old driver reference coverage
# ============================================================================
reference_coverage() {
    echo "=== Part 1: Running reference (old driver) coverage ===" >&2

    if [ "$SKIP_REFERENCE" = true ]; then
        echo "(skipping reference run, reusing existing results)" >&2
    else
        "$PROJECT_ROOT/scripts/old_odbc/coverage_full.sh" || echo "WARNING: reference coverage exited with non-zero status" >&2
    fi

    {
        echo "## 1. Old Driver Coverage (New Tests on Reference Driver)"
        echo ""
        echo "Coverage of the old (reference) ODBC driver source code exercised by our new test suite."
        echo ""

        local summary_file="$PROJECT_ROOT/coverage_report/summary.txt"
        if [ -f "$summary_file" ]; then
            local line_info func_info
            line_info=$(grep "lines" "$summary_file" | head -1 || true)
            func_info=$(grep "functions" "$summary_file" | head -1 || true)

            local line_pct line_hit line_total func_pct func_hit func_total
            if [ -n "$line_info" ]; then
                line_pct=$(echo "$line_info" | sed -E 's/.*: ([0-9.]+)%.*/\1/')
                line_hit=$(echo "$line_info" | sed -E 's/.*\(([0-9]+) of.*/\1/')
                line_total=$(echo "$line_info" | sed -E 's/.*of ([0-9]+).*/\1/')
            fi
            if [ -n "$func_info" ] && [[ "$func_info" != *"no data"* ]]; then
                func_pct=$(echo "$func_info" | sed -E 's/.*: ([0-9.]+)%.*/\1/')
                func_hit=$(echo "$func_info" | sed -E 's/.*\(([0-9]+) of.*/\1/')
                func_total=$(echo "$func_info" | sed -E 's/.*of ([0-9]+).*/\1/')
            fi

            echo "| Metric | Coverage | Hit / Total |"
            echo "|--------|----------|-------------|"
            if [ -n "${line_pct:-}" ]; then
                echo "| Line coverage | ${line_pct}% | ${line_hit} / ${line_total} |"
            fi
            if [ -n "${func_pct:-}" ]; then
                echo "| Function coverage | ${func_pct}% | ${func_hit} / ${func_total} |"
            fi
        else
            echo "*No coverage summary found at \`coverage_report/summary.txt\`.*"
            echo "*Run without \`--skip-reference\` or run \`scripts/old_odbc/coverage_full.sh\` first.*"
        fi
        echo ""
    } >> "$REPORT_FILE"
}

# ============================================================================
# Part 2: New driver fully-enabled odbc-api tests
# ============================================================================
new_driver_full_tests() {
    echo "=== Part 2: Running fully-enabled odbc-api tests on new driver ===" >&2

    local BUILD_DIR="$PROJECT_ROOT/odbc_tests/cmake-build-report"
    local JUNIT_FILE="$REPORT_DIR/new_driver_odbc_api_results.xml"

    if [ "$SKIP_NEW_DRIVER" = true ]; then
        echo "(skipping new driver run, reusing existing results)" >&2
    else
        echo "Building new driver..." >&2
        (cd "$PROJECT_ROOT" && cargo build)

        if [[ "$OSTYPE" == "darwin"* ]]; then
            DYLIB_EXT="dylib"
            NPROC=$(sysctl -n hw.ncpu)
        else
            DYLIB_EXT="so"
            NPROC=$(nproc)
        fi

        export DRIVER_PATH="$PROJECT_ROOT/target/debug/libsfodbc.${DYLIB_EXT}"
        export PARAMETER_PATH="${PARAMETER_PATH:-$PROJECT_ROOT/parameters.json}"

        echo "Building odbc-api tests with FORCE_RUN_NOT_IMPLEMENTED=ON..." >&2

        rm -rf "$BUILD_DIR"
        mkdir -p "$BUILD_DIR"

        local ODBC_PREFIX
        if [[ "$OSTYPE" == "darwin"* ]]; then
            ODBC_PREFIX=$(brew --prefix unixodbc)
            ODBC_LIBRARY="${ODBC_PREFIX}/lib/libodbc.dylib"
            ODBC_INCLUDE_DIR="${ODBC_PREFIX}/include"
        else
            ODBC_LIBRARY="/usr/lib/x86_64-linux-gnu/libodbc.so"
            ODBC_INCLUDE_DIR="/usr/include"
        fi

        (cd "$PROJECT_ROOT/odbc_tests" && \
            cmake -B "$BUILD_DIR" \
                -DCMAKE_CXX_FLAGS="-O0" \
                -DCMAKE_BUILD_TYPE=Debug \
                -D ODBC_LIBRARY="$ODBC_LIBRARY" \
                -D ODBC_INCLUDE_DIR="$ODBC_INCLUDE_DIR" \
                -D DRIVER_TYPE=NEW \
                -D FORCE_RUN_NOT_IMPLEMENTED=ON \
                . && \
            cmake --build "$BUILD_DIR" -- -j "$NPROC")

        echo "Running fully-enabled odbc-api tests on new driver..." >&2
        (cd "$PROJECT_ROOT/odbc_tests" && \
            ctest --test-dir "$BUILD_DIR" \
                --output-on-failure \
                --output-junit "$JUNIT_FILE" \
                -j "$NPROC" \
                -C Debug \
                2>&1 | tee "$REPORT_DIR/new_driver_ctest_output.txt") || true
    fi

    {
        echo "## 2. New Driver: Fully-Enabled odbc-api Tests"
        echo ""
        echo "All odbc-api tests run on the new driver with \`SKIP_NEW_DRIVER_NOT_IMPLEMENTED\` disabled."
        echo "This shows actual implementation progress — tests that fail here indicate unimplemented or broken functionality."
        echo ""

        if [ -f "$JUNIT_FILE" ]; then
            parse_ctest_results "$JUNIT_FILE"
            list_failed_tests "$JUNIT_FILE"
        elif [ -f "$REPORT_DIR/new_driver_ctest_output.txt" ]; then
            echo '```'
            tail -5 "$REPORT_DIR/new_driver_ctest_output.txt"
            echo '```'
        else
            echo "*No test results found. Run without \`--skip-new-driver\`.*"
        fi
        echo ""
    } >> "$REPORT_FILE"
}

# ============================================================================
# Part 3: Static analysis — count SKIP_NEW_DRIVER_NOT_IMPLEMENTED occurrences
# ============================================================================
skip_macro_analysis() {
    {
        echo "## 3. SKIP_NEW_DRIVER_NOT_IMPLEMENTED Usage"
        echo ""
        echo "Number of \`SKIP_NEW_DRIVER_NOT_IMPLEMENTED()\` calls per odbc-api test file:"
        echo ""
        echo "| File | Count |"
        echo "|------|-------|"

        local total=0
        while IFS=: read -r file count; do
            file=$(echo "$file" | sed "s|$PROJECT_ROOT/||")
            count=$(echo "$count" | tr -d ' ')
            echo "| \`$file\` | $count |"
            total=$((total + count))
        done < <(grep -rc 'SKIP_NEW_DRIVER_NOT_IMPLEMENTED' "$PROJECT_ROOT/odbc_tests/tests/odbc-api/" 2>/dev/null | grep -v ':0$' | sort)

        echo "| **Total** | **$total** |"
        echo ""
    } >> "$REPORT_FILE"
}

# ============================================================================
# Main
# ============================================================================
echo "Generating ODBC weekly progress report for $DATE"
echo "Report will be written to: $REPORT_FILE"
echo ""

cat > "$REPORT_FILE" << EOF
# ODBC Implementation Progress Report

**Date:** $DATE
**Branch:** $(cd "$PROJECT_ROOT" && git rev-parse --abbrev-ref HEAD)
**Commit:** $(cd "$PROJECT_ROOT" && git rev-parse --short HEAD)

---

EOF

reference_coverage
new_driver_full_tests
skip_macro_analysis

{
    echo "---"
    echo ""
    echo "*Generated by \`scripts/odbc/generate_weekly_report.sh\`*"
} >> "$REPORT_FILE"

echo ""
echo "========================================"
echo "Report generated: $REPORT_FILE"
echo "========================================"
echo ""
cat "$REPORT_FILE"
