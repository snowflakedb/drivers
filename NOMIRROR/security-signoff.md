# Security sign-off gate

Merging to `main` (or a `release/*` branch) puts a change on the public
mirror at `snowflakedb/universal-driver` on the next Copybara run. Merge is
therefore effectively publication, and a change that needs pre-disclosure
review has to be held until someone has explicitly cleared it for the
public.

That hold is the `security-signoff` gate, implemented by
`snowflake-eng/security-signoff-action`.


## How it works

1. A PR that may need security scrutiny gets the
   `security-signoff-required` label.
2. On PR and review events, `.github/workflows/security-signoff.yml`
   evaluates the PR and publishes a `security-signoff` **commit status**.
3. The status is **success** when the label is absent, or when a security
   partner other than the author has an `APPROVED` review **on the PR's
   current head commit**. Otherwise it is **failure**.
4. Branch protection requires that status, so a labeled PR cannot merge
   until a partner clears it.

"Cleared" means **cleared for public disclosure**, not merely "the code
looks correct". A partner signing off is asserting that it is fine for this
change, its commit messages, and its PR description to become public on the
next mirror run.

### Clearance is bound to the reviewed commit

An approval only counts for the exact commit it was made against. Any new
push re-blocks the PR until a partner re-approves. This gives
dismiss-on-change behaviour scoped to this one gate, without enabling
repo-wide "dismiss stale reviews".

### The roster is read from the base branch

`.github/security-partners.yml` is always read from the PR's
**base** branch, never the head, so a pull request cannot edit the list that
governs its own sign-off.


## How to use it

### Getting a labeled PR cleared

Ask one of the partners in `.github/security-partners.yml` to
review. When they approve, the status flips to success within a minute — the
workflow triggers on `pull_request_review`, so it does not wait for another
push. If you push again afterwards, you need a fresh approval.

A non-partner approval does not clear the gate, and neither does your own
approval of your own PR.

### Writing the PR

The gate controls *when* a change becomes public; it does not sanitize
*what* becomes public. Commit messages and the PR description are mirrored
verbatim. Follow `.ai/review/universal-driver-security-disclosure.yaml`:
a bare `SNOW-\d+` reference, no CVE/CWE/GHSA identifiers, no narration of
what was wrong or how it could be triggered.

### Adding or removing a partner

Edit `.github/security-partners.yml` on `main`, plus any `release/*` branch
that needs the same roster (each branch is its own base). Entries are
individual logins, matched case-insensitively; `@org/team-slug` references are
not resolved. Use the `_snow` login the person actually reviews with in
`snowflake-eng` — a public-org `sfc-gh-*` account cannot review on this repo
and so could never clear a gate. Verify the login resolves and is an org
member:

```bash
gh api /users/<login> --jq .login
gh api /orgs/snowflake-eng/members/<login> --silent && echo member
```

### Turning the gate off for an emergency

Remove the `security-signoff-required` label; the status goes green on the
next event. That is deliberately a visible, audited action rather than a
config change.


## Merge queue

This repo runs a merge queue, and a queue re-evaluates the same required-check
list against the merge-group head rather than only against the PR. GitHub is
explicit that a required context which never reports there blocks the merge:
workflows performing required checks must add the `merge_group` trigger,
otherwise "the merge will fail as the required status check will not be
reported". The entry sits pending until the queue's status-check timeout
elapses — 120 minutes, per the "Protect main" ruleset — is then assumed to have
failed, and is dequeued, so the PR becomes permanently unmergeable rather than
failing visibly.

The verdict cannot be recomputed inside the queue, because it reads PR labels
and reviews and a merge group has neither; `merge_group` carries only the
`checks_requested` activity type. `.github/workflows/security-signoff.yml`
therefore publishes a success status for the merge-group head instead. That is
sound because a PR cannot enter the queue until its required checks are green
on its head commit, so every PR in a group has already been cleared.

Skipping the job with an `if:` condition would not work. A skipped job reports
success as a *check run*, but posts no *commit status*, and `security-signoff`
is a commit status — the context would stay unreported and the entry would time
out exactly as if the trigger were missing.

**`HEADGREEN` grouping is not a hole in this gate.** "Protect main" sets
`grouping_strategy: HEADGREEN`, so a group's verdict comes from the checks on
the group head rather than from every speculative entry. That trades CI cost
against churn *inside* the queue; it does not relax entry into it. GitHub
evaluates a pull request's own required checks before it can be queued, so by
the time a labeled PR is in a group, a partner has already cleared its head
commit. `ALLGREEN` would buy this gate nothing.

There is no configuration-only alternative: GitHub provides no way to require a
check on pull requests but not on merge groups.

Only `main` has a queue. The "Protect release branch" ruleset configures none,
so on `release/*` the merge-group job never fires and is inert.


## Known limits

**The label is the gate.** Everything above assumes the PR carries
`security-signoff-required`. A change that needs disclosure review but never
gets labeled is not gated at all, and no branch-protection setting changes
that. This is the likeliest way the gate fails to fire, and it deserves more
attention than any of the mechanical gaps below.

**Bypass actors can merge past a red status.** Both rulesets grant bypass to
repository role id 5 — `bypass_mode: pull_request` on "Protect main", `always`
on "Protect release branch". Resolving that id to a role name over the API
needs `admin:org`; the ruleset UI shows it directly. Confirm the holder is who
you expect before treating the status as a hard block.

**Revoking clearance after queueing is untested.** If a partner dismisses their
approval once the PR has entered the merge queue, the action posts failure on
the PR head, but whether the queue re-evaluates an entry it has already accepted
is unknown. The window is minutes wide, and closing it would need a queue-level
check rather than anything in this workflow.

**The mirror carries no gate**, by construction — see "Why these files are not
mirrored". Anything that reaches the public repo is already public, so this gate
only ever operates upstream of the outbound sync.


## Why these files are not mirrored

`.github/workflows/security-signoff.yml` and `.github/security-partners.yml`
both have explicit `EXCLUDED_PATHS` entries in `ci/mirroring/copy.bara.sky`.
The workflow `uses:` an org-internal action the public org cannot resolve, and
fork PRs on a public repo get a read-only token that cannot post commit
statuses — so the gate cannot function there and would only produce a failing
job on every PR. Publishing the roster would also advertise exactly who can
authorize a disclosure.

Neither file is moved below a `NOMIRROR/` directory, even though that would
exclude it automatically: GitHub only reads workflows from
`.github/workflows/*.yml`, and the action expects the roster at its default
path. Listing both explicitly keeps every file belonging to the gate in one
reviewable place in the denylist.

Neither of the mirror's automatic safety nets would have caught that action
reference on its own. The outbound `verify_match` guard matches only the
literal `snowflake-eng/universal-driver`, and
`.ai/review/universal-driver-mirror-privacy.yaml` lists `.github` and `ci`
under `excluded_folders`, so the ArcticOwl reviewer never inspects workflow
or Copybara files. Both exclusions above are therefore load-bearing rather
than belt-and-braces.


## Files involved

- `.github/workflows/security-signoff.yml` — the gate; publishes the status.
- `.github/security-partners.yml` — the roster.
- `ci/mirroring/copy.bara.sky` — mirror exclusions.
- `.ai/review/universal-driver-security-disclosure.yaml` — the ArcticOwl
  rules for how a security-relevant change should be worded.


## Remaining steps

Protection lives in two rulesets, not classic branch protection: "Protect main"
(target `~DEFAULT_BRANCH`) and "Protect release branch" (target
`refs/heads/release/*`). Until the status is required on both, this workflow
publishes a verdict that nothing enforces.

- **Require `security-signoff` on "Protect main".** It already requires eleven
  other checks; add this one.
- **Give "Protect release branch" some rules, then require the status there
  too.** That ruleset's `rules` array is currently empty — not merely missing
  required checks, but carrying no pull-request requirement and no checks of any
  kind. The branches API reports `protected: true` for `release/*`, which only
  means that some ruleset matches the ref, so protection read that way is
  misleading. Release branches are where security backports land, which makes
  this the widest of the gaps. Enforcement has to happen in this repo: the gate
  is deliberately absent from the mirror, changes reach the mirror only through
  the outbound Copybara sync, and releases are cut from the mirror — by the time
  a change is there it is already public.
- **Confirm the bypass actors** on both rulesets are who you expect, per
  "Known limits" above.
- **Seed the roster on existing release branches.** The action reads
  `.github/security-partners.yml` from the PR's base branch. Every current
  `release/*` branch was cut before this change, so none carries the file;
  requiring the status there first would block every labeled PR with no way to
  clear it. Branches cut from `main` afterwards inherit it.
- **Test the revoke-after-queueing case** from "Known limits" above — one
  throwaway PR settles it.
