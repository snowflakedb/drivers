# Meta-skill manifest — which skills get copied

The bootstrap installs two groups of meta-skills into the target
repo. No user choice — they ship together because each is either
a CLI-command wrapper (orchestrators) or a repo-hygiene audit
tool teams will need once the skill set grows.

## The 4 lifecycle orchestrators (CLI-command wrappers)

Each orchestrator auto-triggers from natural-language prompts so
the underlying `sf ai skills` command is reachable without
memorization.

| Skill | What it does | Invoked when |
| --- | --- | --- |
| `author-skill` | Creates a new skill or modifies an existing one via LLM. | User says "create a skill", "new skill", "fix this skill". |
| `generate-eval-set` | Authors/updates the routing-accuracy eval set for a skill. | User says "generate evals for my skill", "add eval set". |
| `run-skill-evals` | Runs routing-accuracy evals via `sf ai skills eval`. | User says "run skill evals", "test skill routing". |
| `migrate-repo-to-skills` | Plans the migration from `.ai/commands/` to `.claude/skills/`. | User says "migrate .ai to skills". |

## The 1 audit skill (repo-hygiene tool)

Does NOT wrap a CLI command. Does NOT auto-trigger during the
adoption journey. Teams invoke it when they want to audit
frontmatter health across their growing skill set — typically
post-migration, or periodically as the skill count grows. Ships
with an eval set because its trigger surface IS routing-accuracy-
evaluated (unlike the orchestrators, which are exempt because
they wrap CLI commands rather than being model-routed at the
decision level).

| Skill | What it does | Invoked when |
| --- | --- | --- |
| `configure-skill-settings` | Audits SKILL.md frontmatter across `.claude/skills/` directories — flags `disable-model-invocation`, paths vs globs, description length, name/directory mismatches, skill visibility/shadowing. | User says "audit my skill settings", "review skill frontmatter", "configure skill settings". |

## Source of truth

Read the snowdev-sourced copies at bootstrap time:

```
<snowdev-root>/.claude/skills/author-skill/
<snowdev-root>/.claude/skills/generate-eval-set/
<snowdev-root>/.claude/skills/run-skill-evals/
<snowdev-root>/.claude/skills/migrate-repo-to-skills/
<snowdev-root>/.claude/skills/configure-skill-settings/
```

Copy each directory wholesale — including `metadata/`, `references/`,
`eval_sets/`, and any other subdirectories. The four orchestrators
do not have `eval_sets/` (exempt from routing-accuracy evals per
the contract); `configure-skill-settings` DOES ship with an
`eval_sets/routing-accuracy.yaml` — copy it.

The skill body must read these source paths at runtime — do NOT
embed their content in this manifest. Snowdev is canonical; when it
changes, fresh bootstraps get the change.

### Non-skill canonical sources

The bootstrap also reads two non-skill files from the same snowdev
tree at runtime — same "canonical, do not embed verbatim here"
contract as the meta-skills and the lifecycle README:

```
<snowdev-root>/.claude/settings.json   (the `hooks` key only;
                                        permissions is repo-specific)
<snowdev-root>/.cursor/hooks.json      (entire file)
```

These are the telemetry-hook canonical sources. See
`telemetry-hooks.md` for requirements and merge semantics.

## Target layout

Land each under the target repo's `.claude/skills/<name>/`:

```
<target-repo>/.claude/skills/author-skill/
<target-repo>/.claude/skills/generate-eval-set/
<target-repo>/.claude/skills/run-skill-evals/
<target-repo>/.claude/skills/migrate-repo-to-skills/
<target-repo>/.claude/skills/configure-skill-settings/
```

No nesting — meta-skills live at the root of `.claude/skills/`
regardless of the target repo's nested layout preference. They're
lifecycle-level, not domain-level.

## Idempotency

For each meta-skill (orchestrator or audit), the audit in Phase 1
should check:

1. Does `<target-repo>/.claude/skills/<name>/SKILL.md` exist?
2. If yes, diff against the snowdev source (including
   subdirectories — `metadata/`, `references/`, `eval_sets/`):
   - **Byte-identical** → "Already installed, skipping."
   - **Content diff but same frontmatter `name`** → Show the diff;
     ask the user overwrite / keep existing / merge.
   - **Frontmatter name mismatch** (extremely unlikely) → Abort and
     ask the user to rename, since `name` is the skill's identity.
3. If no → "Will copy from snowdev source."

Never silent-overwrite. The user in Phase 2 explicitly picks the
resolution for any diverged skill.

## Skills NOT copied by this bootstrap

Snowdev has other skills that are NOT meta-skills and must not be
copied into random target repos:

- `address-github-comments` — workflow skill, opinionated.
- `assign-reviewers` — workflow skill, snowdev-specific reviewer
  policy.
- `cloud-agent-setup` — snowdev-specific Cloud Agent Worker tooling.
- `kubectl-diagnose` — dev-env tool, not lifecycle.
- `merge-pr` — workflow skill, opinionated.
- `snowci-get-runtime-logs` — snowci-specific.

The bootstrap only ever touches the 5 meta-skills (4 orchestrators
+ 1 audit skill) plus the `bootstrap-skills-evals` skill itself
(the one running). It does NOT copy itself into the target repo;
`bootstrap-skills-evals` lives only in snowdev.
