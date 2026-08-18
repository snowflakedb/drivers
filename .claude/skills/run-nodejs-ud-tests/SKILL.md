---
name: run-nodejs-ud-tests
description: >
  Runbook for building and running Node.js Universal Driver (UD) tests. Use
  when you need to run Node.js UD unit or e2e tests via Vitest, or run the
  old-driver reference suite for comparison.
---

## Node.js UD Test Runner

All commands run from the **repo root** unless stated otherwise.

---

## Prerequisites

- Node.js 22+ — use `nvm` to install/select it.
- npm (bundled with Node.js)
- Rust toolchain — `npm run build:core` shells out to `cargo`
- A **writable** npm global prefix — `build:core` finishes with `npm link`

One environment gotcha can still break Step 3 if `cargo` isn't on `PATH`:

```bash
which cargo || export PATH="$HOME/.cargo/bin:$PATH"
```

---

## Step 1 — Credentials (required for e2e tests only)

Unit tests do not need credentials. For e2e, decode credentials **once** (test
scripts never decode):

```bash
cd nodejs
npm run creds:decode
```

Equivalent from the repo root: `./scripts/decode_secrets.sh`. Full setup
procedure (passphrase / 1Password / manual `parameters.json`):

@.claude/rules/ud-credentials.md

E2E helpers resolve credentials from repo-root `parameters.json` by default
(or `PARAMETER_PATH` if set). Env vars (`SNOWFLAKE_TEST_*`) remain a fallback.

---

## Step 2 — Install dependencies

```bash
cd nodejs
npm install
```

---

## Step 3 — Build the native core

Compiles the `nodejs_bridge` crate (and transitively `sf_core`) into
`_build/snowflake-sdk-core/`, then links it into `node_modules/`:

```bash
cd nodejs
npm run build:core
```

Required for new-driver e2e tests; not needed for `test:e2e-old-driver`. Rerun
after any change under `nodejs_bridge/` or `sf_core/`. Needs `cargo` and a
writable npm prefix — see Prerequisites.

---

## Step 4 — Run tests

### Unit tests (no credentials, no core needed)

```bash
npm run test:unit
# Or directly:
npx vitest run --project unit
```

### E2E tests — new UD driver

```bash
npm run test:e2e
# Or directly:
npx vitest run --project e2e
```

> CI does not run new-driver e2e tests yet — the step is commented out in
> `.github/workflows/test-nodejs.yml` because they still fail. Run them
> locally.

### E2E tests — old driver reference (for comparison)

```bash
npm run test:e2e-old-driver
# Or directly:
npx vitest run --project e2e-old-driver
```

### Specific test file

```bash
npx vitest run tests/unit/error-codes.test.ts
npx vitest run --project e2e tests/e2e/connection.test.ts
```

### Specific test within a file

Add `-t "<test name>"` (matches on test/`it` name, substrings work):

```bash
npx vitest run tests/e2e/connection.test.ts -t "rejects invalid input"
```

### Watch mode (development)

```bash
npm run test:unit -- --watch
```

---

## Key environment variables

| Variable | Required | Purpose |
|---|---|---|
| `PARAMETER_PATH` | No (helpers default to repo-root `parameters.json`) | Override path to credentials file |
| `SNOWFLAKE_NODEJS_E2E_USE_OLD_DRIVER` | No (set by the `e2e-old-driver` project) | Routes `getSnowflakeSDK()` to `snowflake-sdk` (old driver) |

---

## Test timeouts

| Suite | Test timeout | Hook timeout |
|---|---|---|
| `unit` | 1 s | default |
| `e2e` | 30 s | 30 s |
| `e2e-old-driver` | 30 s | 30 s |

---

## Troubleshooting

### `Cannot find module 'snowflake-sdk'`

Run `npm install` in the `nodejs/` directory.

### `Cannot find module 'snowflake-sdk-core-<platform>'`

The native core is not built or not linked. Run `npm run build:core` in the
`nodejs/` directory.

### `build:core` fails spawning `cargo metadata`

`cargo` is not on `PATH` (common in non-login shells):

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

### `build:core` fails on `npm link` (`status: 254`, or `ENOENT` on a `/lib` path)

The crate compiled and only the final link step failed — but that step is what
creates `node_modules/snowflake-sdk-core-<platform>`, so without it every test
that loads the core fails. `npm link` writes to the npm global prefix, which is
read-only for a Nix-installed Node. Switch to a Node installed via `nvm`
(writable prefix by default) rather than working around it:

```bash
nvm install 22
nvm use 22
npm run build:core
```

### Missing test parameter / E2E connection failures

Decode credentials once (do **not** expect `test:e2e` to decode):

```bash
cd nodejs && npm run creds:decode
```

Or from repo root: `./scripts/decode_secrets.sh`. Confirm `parameters.json`
exists at the repo root, or set `PARAMETER_PATH` / the specific
`SNOWFLAKE_TEST_*` env var.

### New-driver e2e failures that pass under `test:e2e-old-driver`

Expected for most tests — most data types are not yet mapped in
`nodejs_bridge`. Don't chase every failure: focus only on tests related to
the current working context, and compare against the old-driver run before
treating one of those as a regression.
