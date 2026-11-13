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
- JDBC uses these same categories. A user-facing or internal JDBC change gets a fragment under `jdbc/changelog/unreleased/`.

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

## Cite a .NET behavior difference

A .NET behavior difference that ships as is has `status: allowed` in
`dotnet/behavior_differences/<area>/<slug>.yaml`. Every such entry is listed
by at least one .NET fragment, so the difference reaches customers in the
release notes. The pull request that sets `allowed` adds the fragment unless
one already lists the entry:

```markdown
---
category: breaking
behavior_differences: [BD#select-rows-affected-reports-row-count]
---

- Changed `ExecuteNonQuery` and `RecordsAffected` to return the row count of a SELECT instead of -1. (snowflakedb/drivers#42)
```

The bullet describes the difference in public API terms, without the
registry's internal `rationale`. One fragment may list several entries, and
an entry may appear in more than one fragment.

`check_changelog.py` fails when a `dotnet/behavior_differences/` entry has
`status: allowed` and no fragment under `dotnet/changelog/unreleased/` or a
.NET version folder lists it, and when a fragment lists an id with no
registry entry. Any fragment that lists the entry counts, including one an
earlier pull request added; the check does not require the fragment to be
new in the pull request that sets `allowed`. Other components do not accept
`behavior_differences:` yet.

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
