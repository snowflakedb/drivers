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

Python coverage is uploaded only when the run includes all PR cells and all
their coverage artifacts are valid and present. A merge-queue-only scope that
does not contain the complete PR matrix therefore has no Python upload. A
failed or missing report fails the upload job instead of publishing a partial
Python percentage. A workflow running only packaging, FIPS or reference tests
does not necessarily run the coverage-producing Python suite.

Changing the Python PR cells changes what the percentage measures. Establish
and verify a new main baseline when the cells change; comparing against an old
matrix can produce a misleading delta. The other drivers currently upload
their available matrix reports, which may differ between PR and main scopes;
their separate flag checks do not by themselves make those matrices equivalent.
Matching cells also cannot eliminate differences from test selection, flaky
tests or environmental changes.

## Initial baseline and verification

After enabling carryforward, seed a report for each driver on a `main` commit
using Driver CI and the internal Jenkins Codecov upload. A manual `driver=all`
dispatch runs the nightly matrix and can be expensive. Dispatching tests alone
does not trigger the Jenkins upload: arrange that separately for the same SHA,
and confirm the Jenkins log identifies the intended Driver CI run and reports.
The Python upload uses the PR cells even in a nightly dispatch. The other
drivers' nightly matrix can differ from their normal PR and main matrices.

In the Dedicated Codecov UI, verify each seeded flag and then open a PR that
changes Python. Confirm the displayed head and base SHAs, the Python flag's
coverage on both commits, and the `codecov/project/python` delta. Repeat this
after changing the coverage matrix or if a report is missing. A successful
upload alone does not prove that Codecov processed the report or selected the
intended base.
