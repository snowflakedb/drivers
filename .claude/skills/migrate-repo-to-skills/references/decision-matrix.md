# Decision matrix — classifying legacy `.ai/` artifacts

Apply this matrix to every source artifact found in Phase 1. The output
is a proposed target path + action, which feeds the Phase 3 proposal.

## Classification table

| Source shape | Example | Target | Rationale |
| --- | --- | --- | --- |
| Single `.md` with full how-to body (step-by-step workflow, includes decision logic) | `.ai/commands/merge-pr.md` | `.claude/skills/<name>/SKILL.md` | A how-to is a procedure; procedures are skills. |
| Directory with `SKILL.md` + supporting files | `.ai/commands/snowci/graphite-pr-mgmt/` | `.claude/skills/<name>/` (wholesale move, including `references/`, `metadata/`) | Already in skill shape; just rename the path. |
| Single `.md` that's a thin pointer to an external doc | `.ai/commands/check-docs-for-branch.md` pointing to `.ai/commands/check-docs-for-branch/SKILL.md` | Delete the pointer; migrate the real content directly. | Pointers under the new layout are `.cursor/skills/` concerns; the Claude side owns the canonical file. |
| `.ai/context/*.md` naming a directory scope (e.g., `documentation-ecosystem.md`) | `.ai/context/doc-check-config.md` | `.claude/skills/<owning-skill>/references/<name>.md` OR a new ancestor `CLAUDE.md` entry | Context files live as references when consumed by a skill, or as CLAUDE.md when they're directory orientation. |
| `.ai/context/*.md` that's a rule/policy (hard constraint, "you MUST / never do X") | `.ai/context/gh-pr-checks-duplicates.md` | `.claude/rules/<rule>.md` (user picks exact filename) | Rules are hard constraints; they live in `.claude/rules/` so Cursor/Claude load them unconditionally. Ask the user per-artifact — never guess. |
| Already duplicated in `.claude/skills/<x>/` with identical content | `.ai/commands/foo.md` and `.claude/skills/foo/SKILL.md` are byte-identical | Delete the `.ai/` copy | Redundant. |
| Already duplicated in `.claude/skills/<x>/` but content has diverged | `.ai/commands/foo.md` and `.claude/skills/foo/SKILL.md` differ | **Flag. Ask user.** | Never silent-delete — the user must pick which is authoritative or how to merge. |
| Boilerplate / deprecated / empty / placeholder | README stubs that just say "see other dir" | Delete | Not worth preserving. |
| `.ai/README.md` | the root README in `.ai/` | Leave alone (out of scope) | Not under `commands/` or `context/`. |

## Tie-breakers

When an artifact plausibly fits two buckets, prefer the most actionable
target — in this order:

1. **Skill** (procedure + knowledge together) — highest utility for
   the agent. Pick this if the file tells someone *how* to do
   something.
2. **Rule** (hard constraint) — pick this when the content is "don't
   do X" / "always do Y" and the rule applies across many contexts.
3. **CLAUDE.md entry** (directory orientation) — pick this for
   "here's what lives in this directory" overviews.
4. **Delete** — pick this only when the content is redundant,
   deprecated, or empty.

Never drop content without an explicit delete verdict. If in doubt,
propose a skill and let the user downgrade in Phase 3.

## Namespacing: from source subdir to target layout

The source layout uses subdirs under `.ai/commands/` for organization
(`dev-env/`, `snowci/`, etc.). How these map depends on the layout
chosen in Phase 2:

- **Layout (a) mirror**: the subdir becomes a target subdir —
  `.ai/commands/dev-env/foo.md` → `.claude/skills/dev-env/foo/SKILL.md`.
  Root-level files stay at `.claude/skills/<name>/`.
- **Layout (b) flat**: everything lands at `.claude/skills/<name>/`.
  If two source subdirs both have `init.md`, only those colliding
  basenames go into subdirs (hybrid nesting). Everything else stays
  flat.
- **Layout (c) custom**: user edits per row. Audit still produces a
  default proposal per (a) for starting point.

For layout (b) under a collision, the skill `name:` frontmatter field
stays the basename — Cursor's docs confirm the leaf folder is the
skill identity.

## `.ai/context/` → `.claude/skills/<x>/references/` mapping

A context file that's referenced by exactly one skill becomes that
skill's reference. A file referenced by many skills is either:

- **Still a reference** — copy into the most-referencing skill's
  `references/`, and update the others to point to that canonical
  location (update their `references/` symlinks or in-body paths).
- **A rule** — if the content is policy rather than how-to detail,
  promote it to `.claude/rules/` instead.

Always ask the user for the edge cases. Don't invent a canonical home
without approval.
