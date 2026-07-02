# Layout options — Phase 2 presentation script

Before rendering the full mapping table in Phase 3, the skill must ask
the user to pick a target layout. This reference defines the exact
presentation. **No default** — user must pick explicitly.

## Step 1 — show the inventory

Render a short preamble so the user can make an informed choice:

```
Found N skills across M top-level subdirs:
  - <subdir>/ (X skills)
  - <subdir>/ (Y skills)
  - (root) (Z loose files)

Plus: A .ai/context/ files, B CLAUDE.md references to .ai/ paths,
C files flagged for `sf ai rules build` sweep.
```

Do NOT offer a recommendation. Repo size, team ownership model, and
personal preference all factor in; guessing wrong leads to a
migration the user has to undo.

## Step 2 — present the three options

Use exactly these labels and descriptions:

**(a) Mirror source structure.**
Each `.ai/commands/<subdir>/<name>.md` becomes
`.claude/skills/<subdir>/<name>/SKILL.md`. Root-level files stay at
`.claude/skills/<name>/`. Preserves team/area namespacing from the
legacy layout. Good choice for larger repos (>30 skills) where
subdirs reflect ownership boundaries.

Example for this repo:
```
.ai/commands/dev-env/foo.md   → .claude/skills/dev-env/foo/SKILL.md
.ai/commands/snowci/bar.md    → .claude/skills/snowci/bar/SKILL.md
.ai/commands/baz.md           → .claude/skills/baz/SKILL.md
```

**(b) Flatten to root.**
Every skill lands at `.claude/skills/<name>/SKILL.md`. Simple and
discoverable — one `ls .claude/skills/` shows every skill. If two
source subdirs have the same basename (e.g., `dev-env/init.md` AND
`snowci/init.md`), only those colliding names get hybrid-nested:

```
.ai/commands/dev-env/foo.md       → .claude/skills/foo/SKILL.md
.ai/commands/snowci/bar.md        → .claude/skills/bar/SKILL.md
.ai/commands/dev-env/init.md      → .claude/skills/dev-env/init/SKILL.md  (collision)
.ai/commands/snowci/init.md       → .claude/skills/snowci/init/SKILL.md  (collision)
```

Good choice for smaller repos (<30 skills) or when the source subdir
structure is incidental rather than meaningful.

**(c) Custom.**
The Phase 3 proposal table defaults to (a) but each row is editable.
Pick this when most skills should mirror, but a few belong elsewhere.

## Step 3 — detect and surface collisions (for option b)

Before the user picks, compute which basenames would collide under
(b). If any exist, show them up front so the user understands option
(b) won't be fully flat:

```
Heads-up: picking (b) would nest these colliding basenames:
  - init (dev-env/, snowci/)
  - status (dev-env/, snowci/)
Everything else stays flat. (This is the "hybrid nesting" rule.)
```

If no collisions, say so — (b) will be fully flat.

## Step 4 — block on the pick

Ask: "Pick (a), (b), or (c). No default — please choose."

Wait for the user's response. If they hedge ("whatever you think
best", "you decide"), push back: "The choice depends on team
ownership — please pick. I can explain each option further if
needed."

Only proceed to Phase 3 once the user has explicitly picked one
letter.

## Step 5 — confirm and proceed

Echo the pick back so the user sees it recorded:

```
Layout: (a) mirror source structure.
Proceeding to proposal.
```

Then render the proposal in Phase 3 using the chosen layout.
