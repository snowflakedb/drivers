# Configure Mode

Guides skill authors through frontmatter behavior options via
conversation. Can be run immediately after creation or at any point
later.

**Configure never modifies skill content — only frontmatter.** All
changes are made via the Edit tool directly on the SKILL.md file;
there is no CLI command for CONFIGURE mode.

---

## Available Options

| Option | Type | Effect |
|---|---|---|
| `disable-model-invocation: true` | bool | Command-only; never auto-triggered by model |
| `user-invocable: false` | bool | Hidden from listings; loads via path matching only |
| `paths: ["**/module/**"]` | list | Restricts loading to files matching these globs |
| `argument-hint: "..."` | string | Short hint shown to user on invocation |

---

## Workflow

1. **Identify the target folder** — ask if not provided.

2. **List all skills** with their current effective behavior:
   ```
   <module>/.claude/skills/
   ├── write-tests        user-invocable; model can also auto-trigger
   └── build-project      user-invocable; model can also auto-trigger
   ```

3. **Discover existing skills** with Glob using pattern
   `**/.claude/skills/*/SKILL.md`. Cross-reference with the target
   module path to identify sibling directories that are not covered.

4. **Ask about command-only behavior:**
   > "Which skills should only respond to explicit `/name` invocations?"

   → Apply `disable-model-invocation: true` to those via Edit.

5. **Ask about cross-module sharing:**
   > "Are there sibling directories that also need this skill?"

   → If yes: this is a placement decision, not a frontmatter one.
   Move the skill to the nearest common parent directory. `paths:`
   can narrow loading after placement is correct; it cannot substitute
   for moving the file.

6. **Ask about hidden context skills:**
   > "Are any skills domain context that should auto-load but stay
   > hidden from user listings?"

   → Apply `user-invocable: false` + `paths:` via Edit.

7. **⚠️ Stopping point:** present a summary of all proposed changes
   before applying:
   ```
   Proposed changes:
     write-tests    → disable-model-invocation: true
     build-project  → no change
   Apply these changes?
   ```
   Do not write any frontmatter until the user confirms.

8. **Lint gate** — before writing any frontmatter, check the proposed
   final state against two 🔴 rules:
   - `user-invocable: false` and `disable-model-invocation: true` are
     not both set (nothing can ever invoke the skill)
   - `paths` globs are syntactically valid (only if `paths` is being
     changed)

   If either check fails, report the issue and do not apply until
   resolved.

9. **Apply** frontmatter changes via the Edit tool. Summarize each
   change with its effect and reason.

10. **Re-lint** — run `sf ai skills check <skill-dir>` on each
    modified skill to confirm the frontmatter still passes all rules.
    Fix any 🔴 issues before closing.
