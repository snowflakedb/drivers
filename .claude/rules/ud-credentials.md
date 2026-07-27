# UD Test Credentials Setup

Full procedure for setting up `parameters.json` before running UD integration
or e2e tests. Referenced by `run-*-ud-tests` skills.

---

Check whether `parameters.json` already exists at the repo root:

```bash
ls parameters.json 2>/dev/null && echo "present — skip decode" || echo "missing — decode needed"
```

If **present**, export the path and move on:

```bash
export PARAMETER_PATH="$(pwd)/parameters.json"
```

If **missing**, create it via one of:

```bash
# Option A — 1Password CLI (auto-reads passphrase):
./scripts/decode_secrets.sh [aws|gcp|azure]   # creates parameters.json

# Option B — passphrase via env var:
PARAMETERS_SECRET=<passphrase> ./scripts/decode_secrets.sh

# Option C — create manually at repo root:
# {
#   "testconnection": {
#     "SNOWFLAKE_TEST_ACCOUNT":   "...",
#     "SNOWFLAKE_TEST_USER":      "...",
#     "SNOWFLAKE_TEST_PASSWORD":  "...",
#     "SNOWFLAKE_TEST_DATABASE":  "...",
#     "SNOWFLAKE_TEST_SCHEMA":    "...",
#     "SNOWFLAKE_TEST_WAREHOUSE": "...",
#     "SNOWFLAKE_TEST_ROLE":      "..."
#   }
# }

export PARAMETER_PATH="$(pwd)/parameters.json"
```

<!-- sync-target: this file is a reference doc loaded on demand via @ include in
     skills. It is NOT alwaysApply, so no .cursor/rules counterpart is needed.
     Edit this file only. -->
