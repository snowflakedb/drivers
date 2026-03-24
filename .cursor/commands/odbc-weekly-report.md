Generate the weekly ODBC implementation progress report.

## Instructions

Run the report generation script and present the results.

### Step 1: Run the report script

Execute `./scripts/odbc/generate_weekly_report.sh` from the project root.

If the old driver reference coverage step fails or takes too long (it requires Docker and clones the old ODBC repo), you can re-run with `--skip-reference` and note that the reference coverage section is not available.

If the new driver build or tests fail, capture the error output and include it in the summary.

### Step 2: Read and summarize the report

Read the generated report file at `weekly_report/report.md`.

Present the report contents to the user in a clean format. Highlight:
- Old driver coverage percentages (line and function coverage)
- New driver odbc-api test results: total, passed, failed, skipped
- The list of failed tests (if any) grouped by category
- The SKIP_NEW_DRIVER_NOT_IMPLEMENTED usage count as a measure of remaining work
