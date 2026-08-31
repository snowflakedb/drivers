---
name: run-python-ud-tests
description: >
  Runbook for building and running Python Universal Driver (UD) tests. Use
  when you need to compile the Rust core (sf_core), set up the hatch
  environment, and execute Python UD unit/integ/e2e tests. Also use for:
  "RuntimeError: Couldn't load core driver dependency", "libsf_core not
  found", SKIP_CORE_BUILD, "hatch run dev:unit", "maturin develop", or
  running an adhoc script when credentials or the compiled core are
  unavailable.
---

This content lives in the knowledge base. See `/sdet-non-monorepo:kb tools python-ud-tests` —
`snowflake-eng/dev-platform-kb/.ai/commands/sdet-non-monorepo/kb/references/tools/python-ud-tests.md`.

<!-- Retire this redirect once no citation of `run-python-ud-tests` remains:
     grep -rn 'run-python-ud-tests' --include='*.md' . -->
