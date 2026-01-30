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

# Get coverage rates from root element
line_rate = float(root.get('line-rate', 0)) * 100
branch_rate = float(root.get('branch-rate', 0)) * 100

# Count lines and branches across all packages
lines_valid = 0
lines_covered = 0
branches_valid = 0
branches_covered = 0

for package in root.findall('.//package'):
    for cls in package.findall('.//class'):
        for line in cls.findall('.//line'):
            lines_valid += 1
            if int(line.get('hits', 0)) > 0:
                lines_covered += 1
            
            # Count branches if present
            if line.get('branch') == 'true':
                condition_coverage = line.get('condition-coverage', '')
                if condition_coverage:
                    # Format: "100% (2/2)" or "50% (1/2)"
                    import re
                    match = re.search(r'\((\d+)/(\d+)\)', condition_coverage)
                    if match:
                        branches_covered += int(match.group(1))
                        branches_valid += int(match.group(2))

# Calculate actual percentages from counts (more accurate than root attributes)
if lines_valid > 0:
    line_rate = (lines_covered / lines_valid) * 100

# Write summary in lcov format (compatible with existing benchstore parser)
with open(summary_file, 'w') as f:
    f.write(f"lines......: {line_rate:.1f}% ({lines_covered} of {lines_valid} lines)\n")
    f.write(f"functions..: no data found\n")  # Cobertura doesn't track function coverage the same way
    if branches_valid > 0:
        branch_pct = (branches_covered / branches_valid) * 100
        f.write(f"branches...: {branch_pct:.1f}% ({branches_covered} of {branches_valid} branches)\n")
    else:
        f.write(f"branches...: no data found\n")

print(f"")
print(f"Old Python Connector Coverage Summary")
print(f"=" * 40)
print(f"  Line coverage:   {line_rate:.1f}% ({lines_covered}/{lines_valid} lines)")
if branches_valid > 0:
    print(f"  Branch coverage: {(branches_covered/branches_valid)*100:.1f}% ({branches_covered}/{branches_valid} branches)")
print(f"")
print(f"Summary written to: {summary_file}")
EOF

echo ""
echo "=== Summary file contents ==="
cat "$COVERAGE_DIR/summary.txt"

