# Code Review Design Discipline

Six principles that came up repeatedly during cross-driver review.
Apply them while writing the change, not after.

## 1. Don't ship test seams as global mutable state

When you need test-time behavior to differ from production, in priority
order:

1. **Dependency injection through configuration** — pass the overridable
   behavior as a field on a struct that's already flowing through the
   system. Zero global state.
2. **Compile-time substitution** via `cfg(any(test, feature = "test-utils"))`
   defaults. No runtime cost, no atomic loads, no unreachable production
   branches.
3. **Per-test scoping** (e.g. `tokio::task_local!`) when DI is awkward.
4. **Process-global state**, only if cfg-gated so production can't see it.
   Use `Relaxed` ordering for kill switches; `SeqCst` is overkill.

If the codebase already has a DI seam (`Box<dyn …>`, `Arc<dyn …>`,
function-pointer field), plumb it through instead of adding a static.

```rust
// ❌ Process-wide irreversible kill switch in production code
pub(super) static BROWSER_LAUNCH_DISABLED: AtomicBool = AtomicBool::new(false);
fn default_launch() -> Launcher {
    Box::new(|url| async move {
        if BROWSER_LAUNCH_DISABLED.load(SeqCst) { return; }  // ← read on every prod login
        webbrowser::open(url)
    })
}

// ✅ Config-injected, cfg-defaulted, no global state
pub(crate) browser_launcher: Option<Arc<dyn Fn() -> Launcher + Send + Sync>>,
// in from_settings():
#[cfg(any(test, feature = "test-utils"))]
let browser_launcher = Some(Arc::new(|| Box::new(|_| async {})));
#[cfg(not(any(test, feature = "test-utils")))]
let browser_launcher = None;
```

## 2. Don't parse the same thing twice

Before adding a parser/builder for a config type, grep for existing
functions that produce the same target from the same inputs. If you find
a parallel pipeline, **delegate to a shared parser** or hoist one out.

Self-check: "if I add one new field, how many places do I touch?"
If the answer is >2 (struct definition + one parser), you have shallow
modules and change amplification. Particularly common smell: typed
config and wire-level config as two parallel views of the same
settings — collapse them.

For `From`-style conversions between near-identical structs, derive
`Clone` and write `cfg.clone()` instead of field-by-field copies that
silently miss new fields.

## 3. Layer boundaries must preserve discriminability

When designing error types across FFI / proto / IPC boundaries, the
outer layer must be at least as expressive as the inner one for the
discriminations callers actually need.

```rust
// ❌ Rich inner enum collapses to opaque string at the boundary
enum OAuthError { BrowserTimeout, StateMismatch, TokenExchange { status: u16 } }
//      ↓ FFI
message AuthenticationError { string detail = 1; }
// Tests downstream now have only substring matching to assert on.

// ✅ Carry a discriminant across the boundary
message AuthenticationError {
    AuthErrorKind kind = 1;  // enum
    string detail = 2;
}
```

If you can't fix the boundary in this PR, **don't `format!("{e:?}")`**
in test infrastructure — preserve the typed error up to the assertion
site. When you're forced into substring matching, write the strictest
pattern you can and track the proper fix as a `TODO(TICKET-N)`.

## 4. Visibility should be internally consistent

`pub field: T` where `T` is `pub(crate)` is functionally broken — external
consumers can't name the type. When tempted to mix:

- Field is part of the public contract → promote the type to `pub`.
- Type is genuinely internal → tighten the field to `pub(crate)`.

Mixed visibility usually means the field is an internal injection seam
(test override, observability hook) that shouldn't be on the public
surface at all. APoSD: information hiding.

## 5. Prefer newtypes over manual `Debug` / `Clone` / `Hash` impls

A non-`Debug` field forces a hand-rolled `Debug` listing every other
field. Future field adds will silently drop from the output with no
compiler reminder — that's change amplification re-introduced inside one
file.

```rust
// ❌ Manual Debug; adding a field requires touching two places
impl fmt::Debug for Cfg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Cfg")
            .field("a", &self.a).field("b", &self.b).field("c", &self.c)
            // someone adds field d — and forgets here. Silent.
            .finish()
    }
}

// ✅ Newtype the offending field; #[derive] stays in charge
struct OpaqueLauncher(Arc<dyn Fn() + Send + Sync>);
impl fmt::Debug for OpaqueLauncher {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { f.write_str("<opaque>") }
}

#[derive(Debug, Clone)]
struct Cfg { a: A, b: B, c: C, launcher: OpaqueLauncher }
```

## 6. Uphold documented contracts when adding match arms

When adding an arm to a `match` whose surrounding code has a documented
uniform contract (e.g. `validate_settings` "collect ALL errors"),
**uphold the contract** — or update the doc to mark the exception.

Empty arms with comments like "X is validated elsewhere" are a smell:
either the work happens here, or the contract has an exception worth
documenting at the function level. Refactor-stale comments (e.g.
"validated by `OldFn`" after the work moved to `NewFn`) are worse than
no comment — fix them when you move the code.

---

These are *design-time* principles. For *end-of-task* self-checks
(scope discipline, assertion strictness, stale comments), invoke the
`end-of-task-self-review` skill.

<!-- sync-target: .cursor/rules/code-review-design-discipline.mdc carries an identical body
     (the full content of this file) plus Cursor-specific frontmatter.
     WHY both files need full content instead of a pointer:
       alwaysApply rules are injected into the agent system prompt at session start.
       A pointer file loads the body via tool call, putting it in conversation history
       where context compaction can silently drop it mid-session. Full content in both
       files keeps the rule in context for the lifetime of any session.
     TO UPDATE: edit this file, copy its complete contents into the body of
       .cursor/rules/code-review-design-discipline.mdc (below the closing ---
       of the Cursor frontmatter), then run: bash scripts/check-ai-rules-sync.sh
     The pre-commit hook (ai-rules-sync) catches drift automatically. -->
