#!/usr/bin/env bash
# Two checks in one script:
#
# 1. RULE SYNC — verifies .cursor/rules/*.mdc body is byte-for-byte identical to the
#    corresponding .claude/rules/*.md canonical source (after stripping frontmatter).
#    alwaysApply rules must carry full content in both files; a pointer file places the
#    body in tool-call history where context compaction can drop it mid-session.
#    TO FIX: edit .claude/rules/<name>.md, copy its full contents into the body of
#    .cursor/rules/<name>.mdc (below the closing --- of the Cursor frontmatter).
#
# 2. SKILL MIRROR — verifies every .claude/skills/*/SKILL.md has a matching pointer
#    file at .cursor/skills/*/SKILL.md. Skills fire on demand so a pointer is safe,
#    but the mirror must exist so Cursor users can invoke the skill too.
#    TO FIX: create .cursor/skills/<name>/SKILL.md as a thin pointer that reads:
#      "The full skill definition is in .claude/skills/<name>/SKILL.md."
#
# Run automatically via the ai-rules-sync pre-commit hook, or manually:
#   bash scripts/check-ai-rules-sync.sh
set -euo pipefail

FAIL=0

# ── 1. Rule sync ──────────────────────────────────────────────────────────────
for claude_file in .claude/rules/*.md; do
    base=$(basename "$claude_file" .md)
    cursor_file=".cursor/rules/${base}.mdc"
    [[ -f "$cursor_file" ]] || continue

    # Strip YAML frontmatter from the .mdc file using a state machine:
    # transition out of frontmatter on the second '---', then print everything
    # including any '---' horizontal rules in the body. A single blank line
    # immediately after the closing '---' is conventional and not part of the
    # canonical body, so drop it. Done in awk (not piped through sed) so the
    # script works on both GNU and BSD sed.
    cursor_body=$(awk '
        BEGIN { done=0; n=0; skipped_blank=0 }
        /^---$/ && !done { n++; if (n==2) { done=1 }; next }
        done && !skipped_blank && /^[[:space:]]*$/ { skipped_blank=1; next }
        done { print }
    ' "$cursor_file")

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

# ── 2. Skill mirror ───────────────────────────────────────────────────────────
for claude_skill in .claude/skills/*/SKILL.md; do
    skill_name=$(basename "$(dirname "$claude_skill")")
    cursor_skill=".cursor/skills/${skill_name}/SKILL.md"
    if [[ ! -f "$cursor_skill" ]]; then
        echo "FAIL: $cursor_skill is missing — every .claude/skills/<name>/SKILL.md"
        echo "      needs a matching pointer at .cursor/skills/<name>/SKILL.md so"
        echo "      Cursor users can invoke the skill."
        echo "      Create it as a thin pointer referencing $claude_skill."
        FAIL=1
    fi
done

if [[ $FAIL -eq 0 ]]; then
    echo "OK: all .cursor/rules/*.mdc files are in sync with .claude/rules/*.md"
    echo "    all .claude/skills/ have matching .cursor/skills/ pointers"
fi
exit $FAIL
