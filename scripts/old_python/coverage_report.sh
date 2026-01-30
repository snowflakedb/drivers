#!/bin/bash

# Generate coverage summary from Python Cobertura XML report.
# Creates a summary.txt file compatible with the benchstore upload script.
#
# This parses the coverage of the OLD snowflake-connector-python code
# that was exercised by our comparable tests.

set -e

echo "=== Generating old Python connector coverage summary ==="

COVERAGE_DIR=/workspace/python_coverage_report

if [ ! -f "$COVERAGE_DIR/coverage.xml" ]; then
    echo "Error: Coverage XML file not found at $COVERAGE_DIR/coverage.xml"
    exit 1
fi

# Activate venv for python
source /workspace/python_coverage_venv/bin/activate

# Parse Cobertura XML to extract coverage statistics
python3 << 'EOF'
import xml.etree.ElementTree as ET
from pathlib import Path

coverage_xml = Path("/workspace/python_coverage_report/coverage.xml")
summary_file = Path("/workspace/python_coverage_report/summary.txt")

tree = ET.parse(coverage_xml)
root = tree.getroot()

# Count lines across all packages
lines_valid = 0
lines_covered = 0

for package in root.findall('.//package'):
    for cls in package.findall('.//class'):
        for line in cls.findall('.//line'):
            lines_valid += 1
            if int(line.get('hits', 0)) > 0:
                lines_covered += 1

# Calculate percentage
line_rate = 0
if lines_valid > 0:
    line_rate = (lines_covered / lines_valid) * 100

# Write summary in lcov format (compatible with existing benchstore parser)
with open(summary_file, 'w') as f:
    f.write(f"lines......: {line_rate:.1f}% ({lines_covered} of {lines_valid} lines)\n")
    f.write(f"functions..: no data found\n")

print(f"")
print(f"Old Python Connector Coverage Summary")
print(f"=" * 40)
print(f"  Line coverage:   {line_rate:.1f}% ({lines_covered}/{lines_valid} lines)")
print(f"")
print(f"Summary written to: {summary_file}")
EOF

echo ""
echo "=== Summary file contents ==="
cat "$COVERAGE_DIR/summary.txt"

