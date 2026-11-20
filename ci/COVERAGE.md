# Driver coverage

Dedicated Codecov reports are uploaded by driver: `rust-core`, `python`,
`jdbc`, `odbc`, `nodejs`, `dotnet`, and `gosnowflake`. Each
`codecov/project/<flag>` check compares that flag's coverage on the PR head
with the same flag on Codecov's selected base commit. The target is the base
coverage, with a one-percentage-point tolerance: a base of 80% permits 79%.
The check is omitted if that flag has no new upload on the head; an absent
check is not evidence that coverage passed. The Codecov PR comment shows
newly uploaded flags; carried-forward flags are hidden there. Component entries
group files by path and are not additional flag checks.

Driver CI runs only the suites selected by changed paths. Coverage for an
unselected driver is carried forward from an earlier report when Codecov can
find it; it is not a new test run. If no driver uploads coverage, Jenkins
skips the upload, so not every commit necessarily has a Codecov report.
Codecov can select an earlier base for comparison. Check the actual base and
head commits and their flag reports in Codecov, especially if a base or a
check is missing. Carryforward requires an existing report and does not fix a
missing initial baseline or a failed upload.

## Python coverage

Python wrapper tests are selected as a suite. Regular cells run unit,
integration and end-to-end tests; pandas cells run the pandas and NumPy tests.
CI uses different matrices for PR, merge queue, main and nightly runs. To make
the Python flag comparable, Codecov receives only the Python PR matrix cells,
even when additional cells run. Those additional tests still run and their test
results remain available, but their coverage is not included in this flag.
Successful main nightlies additionally publish all Python cells under
`python-nightly`, without adding those reports to `python`.

Python coverage is uploaded only when the run includes all PR cells and all
their coverage artifacts are valid and present. A merge-queue-only scope that
does not contain the complete PR matrix therefore has no Python upload. A
failed or missing report fails the upload job instead of publishing a partial
Python percentage. A workflow running only packaging, FIPS or reference tests
does not necessarily run the coverage-producing Python suite.

Changing the Python PR cells changes what the percentage measures. Establish
and verify a new main baseline when the cells change; comparing against an old
matrix can produce a misleading delta. Rust uses the same comparison-scope
policy for its coverage-producing cells, as described below. Separate flag
checks do not by themselves make arbitrary test matrices equivalent.
Matching cells also cannot eliminate differences from test selection, flaky
tests or environmental changes.

## Rust Core coverage

`rust-core` contains only the complete PR coverage population at every
eligible scope: currently Linux x64/AWS and macOS ARM/AWS. Windows and FIPS
jobs still run but do not produce these coverage reports. A merge-queue-only
run contains just Linux/AWS, so it does not publish an incomplete comparison
percentage. PRs with broader scope labels still publish only the two PR cells
to `rust-core`.

The PR-comparable report is frozen after the regular, flaky and live-proxy
tests, before the scheduled-only `large_` tests add coverage. Successful main
nightlies publish separate final reports from all six Linux/macOS coverage
cells across AWS/GCP/Azure to `rust-core-nightly`. Scheduled runs include the
`large_` coverage only in this diagnostic flag; manual main dispatches retain
the existing full-matrix behavior without running those scheduled-only tests.

Both populations require separate completeness manifests and one valid lcov
report with tracked lines per cell. Missing, duplicate, unexpected, empty or
invalid reports fail the upload job. Broader matrix cells and nightly-only
tests cannot enter the PR comparison flag. The test matrices themselves are
not expanded or reduced by this policy.

This changes the Rust comparison population from the earlier all-available
matrix reports. Refresh and verify the `rust-core` main baseline after rollout
before interpreting PR deltas against it.

## Nightly coverage

Successful scheduled Driver CI runs on `main` trigger the internal
`drivers-test-coverage-nightly` uploader after completion. Successful manual
Driver CI dispatches on `main` use the same path, so a baseline can be seeded
without waiting until the next night. The webhook pins the workflow run ID,
attempt and tested SHA. Jenkins always loads its uploader from trusted `main`,
but coverage and the source-file index refer to the tested commit, not a newer
Jenkins checkout. A stale attempt or mismatched run fails; it never falls back
to the latest run. Failed or cancelled nightlies do not refresh the baseline.

The existing schedules run Python at 02:00 UTC, ODBC at 03:00, JDBC at 04:00 and
Rust Core at 05:00. This change does not expand the test matrices or add daily
tests for Node.js, .NET or Go; a main dispatch can run those drivers.
Each completed run updates its driver flags, not a synthetic combined daily SHA.
Carryforward supplies older reports for drivers absent from that commit.

Nightlies refresh the existing driver flags and publish separate
`<driver>-nightly` flags for diagnostics. Python's comparison flag still uses
only the eight current PR cells; `python-nightly` uses all 76 current cells.
Rust's comparison flag uses the two current PR coverage cells;
`rust-core-nightly` uses all six current Linux/macOS cells, including
scheduled-only tests after the comparison snapshot. Python and Rust require
complete manifests and valid reports for both populations. The other drivers
keep both flags for a consistent naming and comparison policy, even when
their collected report sets are identical. They reuse their existing
coverage-producing lanes: JDBC still uploads
Java 21/AWS and ODBC still uploads Rust unit coverage, not all integration
cells. A nightly flag does not imply extra instrumentation or a larger
coverage population for every driver.

Only the original driver flags have PR project checks. Nightly flags have no
PR check, and a carried-forward nightly flag is not evidence that the PR ran
those tests. Use `codecov/project/<driver>`, not the overall or component
percentage, for the scoped comparison. Overall coverage can include the wider
nightly reports. Matrix cells retain their individual names and reports in GHA
artifacts; they are not separate Codecov flags. This avoids a flag for every
OS/version/cloud/environment combination and stale flags when the matrix changes.

## Initial baseline and verification

After enabling carryforward, seed a report for each driver on a `main` commit
using Driver CI and the internal Jenkins Codecov upload. A manual `driver=all`
dispatch runs the nightly matrix and can be expensive. Once the nightly Jenkins
job and completion-event routing are activated, a successful main dispatch also
triggers the upload. Confirm the Jenkins log identifies the intended run,
attempt, SHA and reports.
The Python and Rust comparison uploads use the PR coverage cells even in a
nightly dispatch. Their broader populations use separate nightly flags.

In the Dedicated Codecov UI, verify each seeded flag and then open a PR that
changes Python or Rust. Confirm the displayed head and base SHAs, the driver's
comparison flag on both commits, and the corresponding project delta. For
Rust, confirm two comparison reports and six nightly reports on the seeded
main run, with scheduled-only coverage absent from `rust-core`. Repeat this
after changing the coverage matrix or if a report is missing. A successful
upload alone does not prove that Codecov processed the report or selected the
intended base.

Rollout requires the drivers uploader changes on `main`, then the companion
`jenkins_utils` job/routing change and a `jenkins_dsl_seed` run on
`es-ci-legacy-mainvalidation-001`. Verify that the GitHub event feed delivers
`workflow_run.completed` and webhook-dispatch routes it to the new token;
configuration and unit tests alone do not prove live delivery. Dispatch a driver
on `main`, observe the automatically triggered Jenkins build, and verify its
processed flags in Codecov. Do not declare the daily baseline working before
that end-to-end check.
