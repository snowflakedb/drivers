# Telemetry hooks — requirements, canonical source, merge semantics

Two IDE telemetry hook files must end up in the adopter's repo:
`.claude/settings.json` (Claude Code) and `.cursor/hooks.json`
(Cursor). Both wire the `sf ai __hook` command into editor events
so we can measure adoption, skill-routing accuracy in the wild,
and detect implicit skill invocations. Both are mandatory — the
bootstrap does not offer an opt-out.

Unlike precommit hooks and CI steps (which are requirements +
example + adapt-to-repo), the telemetry hook **commands** are NOT
adaptable. The exact `sf ai __hook --ide <ide> --hook-type <type>`
strings are the spec. What IS repo-specific is the *file shape*
around them: an adopter's `.claude/settings.json` may already
contain a `permissions` block or other hooks the bootstrap MUST
preserve.

## Why these hooks exist

Without them, an adopter repo onboards to the convention but emits
no telemetry. We can't tell whether their devs actually use
skills, whether routing is accurate, or which skills get *implicit*
invocations (see below). Telemetry is what closes the feedback
loop on the contract's verdict thresholds.

The hooks fail silently by design: if `sf` isn't on PATH the
command returns non-zero, but Claude Code / Cursor continue the
prompt. Missing telemetry, never blocked work.

## What the hooks do at runtime

The hooks wire `sf ai __hook` into specific editor events. Each
event runs the configured command synchronously with a short
timeout — fast and non-blocking even when `sf` is missing.

| Event | When it fires | What `sf ai __hook` records |
|---|---|---|
| `UserPromptSubmit` (Claude) / `beforeSubmitPrompt` (Cursor) | Every time the user submits a prompt | Prompt envelope + any explicit skill mentions; feeds adoption metrics (are devs using skills?) and routing-accuracy signal (was the right skill loaded for this prompt?) |
| `PostToolUse` (Claude only) | After every tool invocation Claude makes | Tool name + outcome; combined with `--detect implicit-skill`, infers when a skill's *behavior* was reproduced ad-hoc without the skill being loaded |

**What `--detect implicit-skill` means in plain terms:** the flag
tells the hook to look for tool-use patterns that match an
existing skill's expected behavior even when no skill was
triggered for that prompt. Each implicit detection is a routing
miss — the skill existed and matched the user's intent, but
Claude did the work without loading it. These signals are gold
for identifying trigger-phrase gaps in skill descriptions.

The hook commands do NOT log locally or require any developer
setup beyond `sf` on PATH. They send signals to the `besd` daemon
(port 50414), which forwards them to the central telemetry
pipeline. The redirect to `> /dev/null` (Claude entries only) is
just to keep the editor's status line quiet — the hook's
side effects happen via the daemon, not stdout.

## Canonical source

These two files in snowdev are the canonical specs. **Read at
runtime; do not embed verbatim in this reference** — same model
as the lifecycle README. When snowdev changes, fresh bootstraps
get the change.

- **Claude:** `<snowdev-root>/.claude/settings.json` —
  the `"hooks"` key only. The `"permissions"` key is
  snowdev-specific (allows `Bash(bazel:*)`, `Bash(zstd:*)`, etc.)
  and MUST NOT be copied into adopter repos.
- **Cursor:** `<snowdev-root>/.cursor/hooks.json` — the entire
  file is shared canonical content (top-level `version` and
  `hooks` keys both apply across repos).

Pinned source URLs (for reference; the bootstrap fetches from
the snowdev tree at runtime, not from these URLs):

- `https://github.com/snowflake-eng/snowdev/blob/main/.claude/settings.json#L3-L23`
- `https://github.com/snowflake-eng/snowdev/blob/main/.cursor/hooks.json`

## Requirements per file

### `.claude/settings.json`

| Requirement | Value |
|---|---|
| `hooks.UserPromptSubmit` array contains | An entry whose `command` is exactly snowdev's `sf ai __hook --ide claude --hook-type user-prompt-submit > /dev/null` |
| `hooks.PostToolUse` array contains | An entry whose `command` is exactly snowdev's `sf ai __hook --ide claude --hook-type post-tool-use --detect implicit-skill > /dev/null` |
| `permissions` key | Untouched — preserve the adopter's existing block verbatim, or absent if they don't have one |
| File creation | If `.claude/settings.json` doesn't exist, create it with just the `hooks` block |
| Failure semantics | Hook commands return non-zero when `sf` isn't on PATH; Claude Code logs the failure and continues. No blocking. |

### `.cursor/hooks.json`

| Requirement | Value |
|---|---|
| Top-level `version` | `1` (matches snowdev) |
| `hooks.beforeSubmitPrompt` array contains | An entry whose `command` is exactly snowdev's `sf ai __hook --ide cursor --hook-type before-submit-prompt` |
| Other `hooks.*` keys (if adopter has any) | Untouched — preserve verbatim |
| File creation | If `.cursor/hooks.json` doesn't exist, create it with snowdev's exact content |
| Failure semantics | Same as Claude — non-zero on missing `sf`, Cursor continues |

## Detection states

For each file, classify into one of four states during Phase 1
audit so Phase 3 can propose the right action:

| State | Phase 3 action |
|---|---|
| File missing | Create from snowdev source |
| File exists, no `hooks` key | Add the `hooks` block; preserve other keys |
| File exists, has `hooks` but missing snowdev's command strings | Append snowdev's entries to existing arrays (`UserPromptSubmit`, `PostToolUse`, `beforeSubmitPrompt`); preserve other entries |
| File exists, snowdev's entries already present | Skip — already installed |

The append case is the load-bearing one for adopter-friendliness:
an adopter may have their own pre-existing hooks (e.g., custom
prompt logging). We never overwrite — always merge.

## Detection commands

The bootstrap audit can classify each file with these one-liners:

```bash
# .claude/settings.json — does it exist?
test -f "<repo-root>/.claude/settings.json" && echo "exists" || echo "missing"

# Has it got a hooks key?
jq -e '.hooks' "<repo-root>/.claude/settings.json" > /dev/null && echo "has-hooks" || echo "no-hooks"

# Are snowdev's command strings already present?
grep -F 'sf ai __hook --ide claude --hook-type user-prompt-submit' "<repo-root>/.claude/settings.json"
grep -F 'sf ai __hook --ide claude --hook-type post-tool-use --detect implicit-skill' "<repo-root>/.claude/settings.json"

# .cursor/hooks.json
test -f "<repo-root>/.cursor/hooks.json"
grep -F 'sf ai __hook --ide cursor --hook-type before-submit-prompt' "<repo-root>/.cursor/hooks.json"
```

Both `grep` results must hit for Claude detection to count as
"already installed."

## Adapter-side concerns (out of scope for this skill)

A few things the bootstrap does NOT do, but adopters should know
about:

- **`permissions.allow` for `sf`:** if the adopter enforces a
  Claude Code permissions allow-list, they need `Bash(sf:*)` (or
  equivalent) so the hooks can actually execute. The bootstrap
  doesn't manage permissions — repos own that. Surface this in
  the Phase 3 proposal as a callout when the adopter's existing
  `.claude/settings.json` has a `permissions` block, so they can
  add `Bash(sf:*)` themselves.
- **Hook-command tweaks:** adopters should NOT modify the
  `sf ai __hook ...` command strings. Doing so detaches their
  telemetry from the convention's measurement infrastructure.
  If they need additional hooks (their own prompt logging,
  custom `PostToolUse` analytics), they add those AS NEW
  ENTRIES alongside snowdev's, not replacing them.

## Quality rules

- **Never overwrite the file.** Always merge. Adopter content
  outside snowdev's hooks block (other hooks, permissions,
  custom keys) MUST survive the bootstrap.
- **Never modify the snowdev command strings.** They're the
  spec; tweaking them breaks telemetry contracts.
- **Skip on byte-identical match.** Idempotent re-runs must
  detect "already installed" and propose nothing.
- **Surface the `Bash(sf:*)` permission concern** when the
  adopter has a `permissions.allow` block — tell them in the
  Phase 3 proposal that they may need to add it manually.
