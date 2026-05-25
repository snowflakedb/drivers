# universal-driver — Claude memory

Project-level guidance for Claude Code agents working in this repo.
Keep this file thin; put substantive content in `.claude/rules/*.md` and
`.claude/skills/*/SKILL.md`.

## AI file conventions

Two types of agent-config files live in this repo, with different sync rules:

**Rules** (`.claude/rules/*.md` ↔ `.cursor/rules/*.mdc`)
Both files carry identical body content. `alwaysApply` rules are injected into
the agent system prompt at session start; a pointer file would place the body in
tool-call history where context compaction can silently drop it mid-session.
Edit the `.claude/rules/*.md` file (canonical source), then copy its full body
into the matching `.cursor/rules/*.mdc` (below the Cursor frontmatter). The
pre-commit hook `ai-rules-sync` (or `bash scripts/check-ai-rules-sync.sh`) will
catch any drift. See the `sync-target` comment at the bottom of each
`.claude/rules/*.md` file for per-file instructions.

**Skills** (`.claude/skills/*/SKILL.md` ↔ `.cursor/skills/*/SKILL.md`)
Canonical definition lives in `.claude/skills/`. The `.cursor/skills/` files are
thin pointers that reference the canonical. Skills fire on demand — the agent
reads the canonical file fresh at invocation — so pointer files are safe.

@./.claude/rules/code-review-design-discipline.md
