# Apply Review Rulesets While Writing

The `.ai/review/*.yaml` files are this repo's authoritative coding conventions,
enforced by the AI Code Reviewer bot **at PR time**. Apply them while writing so
the bot has nothing to flag.

Before editing or creating a file, read the `.ai/review/*.yaml` whose
`allowed_folders` + `allowed_file_extensions` match it (and not its
`excluded_folders`), and follow that ruleset's `rule:` blocks. Glob the
directory to discover the current set rather than assuming a fixed list.
Examples: `jdbc/src/test/java/**/*.java` → `universal-driver-java.yaml`
(test naming, `wasNull()`, WireMock). `jdbc/src/main/java/**/*.java` →
`universal-driver-java-impl.yaml` (carriers, no public mutable statics,
bounded Arrow allocators) plus the shared logging/security rulesets.

These layer under `code-review-design-discipline.md`; a direct user instruction
wins over both.

<!-- sync-target: .cursor/rules/apply-review-rulesets.mdc carries this body verbatim plus
     Cursor frontmatter. alwaysApply rules are injected into the system prompt at session
     start, so both files need full content (a pointer would land in droppable tool-call
     history). TO UPDATE: edit this file, copy it below the .mdc frontmatter, then run
     bash scripts/check-ai-rules-sync.sh (also a pre-commit hook). -->
