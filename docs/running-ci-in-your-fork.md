# Running CI in your fork

This guide explains how to run the project's continuous-integration (CI) suite
— including the end-to-end / integration tests — **in your own fork, against
your own Snowflake account**. The same per-PR pipelines that gate changes
upstream run in your fork, on your GitHub Actions infrastructure, so you get the
full signal before you open a pull request.

> The encrypted credential bundle under `.github/secrets/*.gpg` is used only by
> the maintainers' own CI and cannot be decrypted without a passphrase that is
> never published. On a fork it is simply ignored — you supply your own
> credentials as described below. Your secrets stay in your fork; the
> maintainers' credentials are never exposed to fork code.

## 1. Enable GitHub Actions on your fork

GitHub disables Actions on new forks by default. Open the **Actions** tab of
your fork and click **"I understand my workflows, go ahead and enable them."**

## 2. Provide your Snowflake credentials as per-cloud secrets

CI loads connection parameters from a `parameters.json` file (via the
`PARAMETER_PATH` environment variable). In a fork, `scripts/decode_secrets.sh`
builds that file from a **repository secret** instead of the encrypted bundle.

Add one secret per cloud you want to exercise, under
**Settings → Secrets and variables → Actions → New repository secret**:

| Secret name              | Used for the cloud |
| ------------------------ | ------------------ |
| `PARAMETERS_JSON_AWS`    | `aws`              |
| `PARAMETERS_JSON_GCP`    | `gcp`              |
| `PARAMETERS_JSON_AZURE`  | `azure`            |

The **value** of each secret is the full contents of a `parameters.json` for an
account on that cloud:

```json
{
  "testconnection": {
    "SNOWFLAKE_TEST_ACCOUNT":   "your-account",
    "SNOWFLAKE_TEST_USER":      "your-username",
    "SNOWFLAKE_TEST_PASSWORD":  "your-password",
    "SNOWFLAKE_TEST_DATABASE":  "your-database",
    "SNOWFLAKE_TEST_SCHEMA":    "your-schema",
    "SNOWFLAKE_TEST_WAREHOUSE": "your-warehouse",
    "SNOWFLAKE_TEST_HOST":      "your-host.snowflakecomputing.com",
    "SNOWFLAKE_TEST_ROLE":      "your-role"
  }
}
```

Notes:
- The account/role must have privileges to create and drop schemas, tables, and
  PATs (the suite creates and cleans up its own objects).
- The test matrix runs the AWS / GCP / Azure lanes. **To reproduce the full
  upstream pipeline (parity), provide all three secrets.** If you provide only
  some, the lanes for the missing clouds will fail their *Decode secrets* step —
  that is expected. (Automatically skipping unprovisioned clouds is a planned
  enhancement.)
- The JDBC, Node.js, and .NET suites read the `aws` parameters by default, so
  `PARAMETERS_JSON_AWS` is required to run those.
- The secret name must match exactly, uppercase: `PARAMETERS_JSON_<CLOUD>`.

## 3. Trigger CI in your fork

The test workflows run on pushes to `main` (and `release/*`) and on pull
requests. Feature-branch pushes with **no** open pull request do not trigger
them. To run CI in your fork, either:

- push your branch to your fork's `main`, **or**
- open a pull request **within your fork** (your feature branch → your fork's
  `main`).

Opening a pull request from your fork to the upstream repository triggers only a
lightweight, credential-free unit smoke on the upstream side — the full suite
runs in *your* fork, using the trigger above.

## What runs on a fork

- The **full per-PR pipeline** for each driver (`test-python`, `test-rust-core`,
  `test-odbc`, `test-jdbc`, `test-nodejs`, `test-dotnet`) runs at parity with
  upstream, scoped by `detect-changes` to the parts of the tree you touched.
- **Nightly / on-demand jobs are not part of per-PR CI** and do not run on a
  normal fork push: the no-MFA authentication suite and the environment-cleanup
  job are scheduled/dispatch-only.
- The **SPCS** end-to-end suite runs only when a pull request carries the
  `ci:spcs` label, and needs SPCS provisioning on your account.

## Troubleshooting

- **A job fails at *Decode secrets*.** The secret for that cloud is missing or
  misnamed. Confirm you added `PARAMETERS_JSON_<CLOUD>` (uppercase) with valid
  JSON as its value.
- **Nothing ran after I pushed.** You pushed a feature branch with no pull
  request; push to your fork's `main` or open an in-fork pull request (see
  step 3). Also confirm Actions are enabled (step 1).
- **Scheduled workflows never run.** GitHub disables scheduled workflows on
  forks; trigger runs via push or pull request instead.
