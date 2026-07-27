# PR Creation Defaults to Draft

Every pull request created via `gh pr create` (or the GitHub API/UI on this
agent's behalf) MUST be created in **draft** state — pass `--draft` to
`gh pr create` unconditionally, including for follow-up PRs, stacked PRs, and
PRs opened as part of a larger task.

Do not ask the user whether to use `--draft` — apply it by default. Only omit
it when the user explicitly says the PR should be ready for review immediately
(e.g. "open this as a normal PR, not draft").

```bash
# ❌ BAD — opens ready-for-review, notifies reviewers immediately
gh pr create --title "..." --body "..."

# ✅ GOOD — opens draft; the user promotes it when ready
gh pr create --draft --title "..." --body "..."
```

<!-- sync-target: .cursor/rules/pr-creation-draft-default.mdc carries this body verbatim plus
     Cursor frontmatter. alwaysApply rules are injected into the system prompt at session
     start, so both files need full content (a pointer would land in droppable tool-call
     history). TO UPDATE: edit this file, copy it below the .mdc frontmatter, then run
     bash scripts/check-ai-rules-sync.sh (also a pre-commit hook). -->
