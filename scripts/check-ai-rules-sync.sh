#!/usr/bin/env bash
# Verifies that .cursor/rules/*.mdc body content is byte-for-byte identical to the
# corresponding .claude/rules/*.md canonical source (after stripping frontmatter).
#
# Run automatically via the ai-rules-sync pre-commit hook, or manually:
#   bash scripts/check-ai-rules-sync.sh
#
# WHY this check exists:
#   alwaysApply rules must carry full content in both .claude/ and .cursor/ — pointer
#   files place the body in tool-call history where context compaction can drop it
#   mid-session. The .claude/ file is the canonical source; the .cursor/ file carries
#   an identical body plus Cursor-specific frontmatter. This script catches drift.
#
# TO FIX a failure: edit .claude/rules/<name>.md, then copy its full contents into
#   the body of .cursor/rules/<name>.mdc (below the closing --- of the frontmatter).
set -euo pipefail

FAIL=0

for claude_file in .claude/rules/*.md; do
    base=$(basename "$claude_file" .md)
    cursor_file=".cursor/rules/${base}.mdc"
    [[ -f "$cursor_file" ]] || continue

    # Strip YAML frontmatter from the .mdc file using a state machine:
    # transition out of frontmatter on the second '---', then print everything
    # including any '---' horizontal rules in the body.
    cursor_body=$(awk '
        BEGIN { done=0; n=0 }
        /^---$/ && !done { n++; if (n==2) { done=1 }; next }
        done { print }
    ' "$cursor_file" | sed '1{/^$/d}')

    claude_body=$(cat "$claude_file")

    if [[ "$cursor_body" != "$claude_body" ]]; then
        echo "FAIL: $cursor_file body has drifted from canonical $claude_file"
        echo "      Edit $claude_file first, then copy its full contents into the"
        echo "      body of $cursor_file (below the closing --- of the frontmatter)."
        echo "Diff (< canonical  > cursor):"
        diff <(echo "$claude_body") <(echo "$cursor_body") || true
        FAIL=1
    fi
done

if [[ $FAIL -eq 0 ]]; then
    echo "OK: all .cursor/rules/*.mdc files are in sync with .claude/rules/*.md"
fi
exit $FAIL
