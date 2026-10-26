# Changelog workflow

Each driver component keeps in-progress release notes as separate files under
`<component>/changelog/unreleased/`. This avoids conflicts between pull
requests that would otherwise edit the same `CHANGELOG.md` lines.

## Add a release note

Classify the pull request before adding a fragment:

- User-facing changes use `breaking`, `feature`, `change`, or `bugfix`.
- Dependency updates use `dependency`.
- Driver implementation changes that customers cannot observe use `internal`.
- Tests, CI, documentation, and AI configuration do not need a fragment.
- JDBC is in private preview. Do not add a fragment for a JDBC change.

Create `<component>/changelog/unreleased/<slug>.md`. The slug uses lowercase
words separated by hyphens and does not start with a number. The category
lives in the fragment metadata:

```markdown
---
category: bugfix
---

- Fixed connection reset on idle timeout. (snowflakedb/drivers#42)
```

Start the bullet with `Added`, `Fixed`, `Removed`, `Improved`, `Changed`, or
`Bumped`. Every user-facing or internal bullet ends with the pull request link
in the form `(snowflakedb/drivers#N)`. Put the same bullet in the pull
request's final `## Changelog` section.

Do not add a bullet directly to `CHANGELOG.md`. Its `Upcoming Release` section
only points readers to `changelog/unreleased/`.

Validate the result locally. The pre-commit hook runs the same command:

```bash
python3 scripts/check_changelog.py --base origin/main
```

That comparison rejects edits to released sections and released version
folders. It also rejects `internal-improvements`, `test-improvements`, and
`known-issues` under `changelog/unreleased/`. Those categories are valid only
in a version folder that already shipped them. CI passes the pull request's
base commit instead of `origin/main`.

## Cut a release

For a release such as ODBC `v4.2.0`:

1. Create `odbc/changelog/v4.2.0/`.
2. Move the fragments included in the release from
   `odbc/changelog/unreleased/` into that directory without changing their
   contents:

   ```bash
   mkdir odbc/changelog/v4.2.0
   git mv odbc/changelog/unreleased/*.md odbc/changelog/v4.2.0/
   ```

   Move individual files instead when some notes belong to a later release.

3. Optionally add `odbc/changelog/v4.2.0/.release`:

   ```yaml
   date: October 20, 2026
   ```

4. Render the new section:

   ```bash
   python3 scripts/check_changelog.py --render
   ```

5. Validate against the branch from which the release is cut:

   ```bash
   python3 scripts/check_changelog.py --base origin/main
   ```

The renderer adds `## v4.2.0` to `odbc/CHANGELOG.md`, groups bullets by
category, and omits fragment metadata. It preserves older release sections and
leaves fragments not selected for the release under `unreleased/`.

After the release lands, both the version folder and its matching
`CHANGELOG.md` section are frozen. Corrections go through a new unreleased
fragment rather than rewriting shipped history.

## Automation and AI guidance

- `scripts/check_changelog.py` validates and renders changelogs.
- `.github/workflows/validations.yml` runs the comparison against the pull
  request base.
- `.pre-commit-config.yaml` runs the same check against `origin/main`.
- `.claude/rules/changelog-format.md` and
  `.cursor/rules/changelog-format.mdc` give coding agents the same format,
  classification, and release rules.
- `.ai/review/universal-driver-changelog.yaml` applies those expectations
  during pull request review.
