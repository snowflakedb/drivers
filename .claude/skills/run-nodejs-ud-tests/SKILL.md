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

- Node.js 22+
- npm (bundled with Node.js)

> **Note:** The Node.js UD does **not** yet link to the Rust core (`sf_core`).
> The new driver is a stub under development. Unit tests and old-driver
> reference e2e tests work without any Rust build step.

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

## Step 3 — Run tests

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

> CI currently marks e2e tests for the new driver as TODO/skipped until the
> new driver implementation is complete.

### E2E tests — old driver reference (for comparison)

```bash
npm run test:e2e-old-driver
# Or directly:
SNOWFLAKE_NODEJS_E2E_USE_OLD_DRIVER=1 npx vitest run --project e2e
```

### Specific test file

```bash
npx vitest run tests/unit/error-codes.test.ts
npx vitest run --project e2e tests/e2e/connection.test.ts
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
| `SNOWFLAKE_NODEJS_E2E_USE_OLD_DRIVER` | No | Set `1` to run e2e tests via `snowflake-sdk` (old driver) |

---

## Test timeouts

| Suite | Test timeout | Hook timeout |
|---|---|---|
| `unit` | 1 s | default |
| `e2e` | 30 s | 30 s |

---

## Troubleshooting

### `Cannot find module 'snowflake-sdk'`

Run `npm install` in the `nodejs/` directory.

### Missing test parameter / E2E connection failures

Decode credentials once (do **not** expect `test:e2e` to decode):

```bash
cd nodejs && npm run creds:decode
```

Or from repo root: `./scripts/decode_secrets.sh`. Confirm `parameters.json`
exists at the repo root, or set `PARAMETER_PATH` / the specific
`SNOWFLAKE_TEST_*` env var.

### New driver e2e tests are all skipped

Expected — the new Node.js driver implementation is a stub. Use
`test:e2e-old-driver` for reference e2e coverage in the meantime.
