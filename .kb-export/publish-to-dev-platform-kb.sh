#!/usr/bin/env bash
# Publish the drivers digest to snowflake-eng/dev-platform-kb (requires gh auth with org access).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
KB_REPO="${KB_REPO:-snowflake-eng/dev-platform-kb}"
WORKDIR="${WORKDIR:-/tmp/dev-platform-kb}"
BRANCH="${BRANCH:-cursor/drivers-knowledge-base-report-$(date +%Y%m%d)}"

gh auth status -h github.com
rm -rf "$WORKDIR"
gh repo clone "$KB_REPO" "$WORKDIR"
cd "$WORKDIR"
git checkout -b "$BRANCH"
rsync -av "$ROOT/snowflake-eng/dev-platform-kb/knowledge/drivers/" knowledge/drivers/
git add knowledge/drivers/
git commit -m "docs(drivers): add main merge digest for 2026-09-22"
git push -u origin "$BRANCH"
gh pr create --base main --head "$BRANCH" \
  --title "docs(drivers): main merge digest 2026-09-15 – 2026-09-22" \
  --body "Automated digest from snowflakedb/drivers main (head 3d1804efb).

## Summary
- Weekly merge report for Universal Driver / sf_core and wrappers
- Covers features, fixes, parity progress, gaps, and CI notes

## Source
- Export path in snowflakedb/drivers: \`.kb-export/\`"
