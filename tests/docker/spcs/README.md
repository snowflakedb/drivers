# SPCS Auth E2E Probe Image

Self-contained image for the Snowpark Container Services (SPCS) authentication
e2e test. It runs the `spcs_probe` binary (`sf_core/src/bin/spcs_probe.rs`) as an
SPCS **job service**: inside the container, the driver authenticates with the
platform-injected OAuth token at `/snowflake/session/token` — **no user supplied**
(made optional for token auth by SNOW-3647715) — while the driver also attaches
the `SPCS_TOKEN` service identifier from `/snowflake/session/spcs_token`
(SNOW-3007075, gated by `SNOWFLAKE_RUNNING_INSIDE_SPCS`). It then runs
`SELECT CURRENT_USER()`. The probe exits `0` on success / non-zero on failure,
so the orchestrating `EXECUTE JOB SERVICE` reports `DONE` vs `FAILED`.

## What's in the image

- **Builder stage** (`rust:1.88-bookworm`): compiles `spcs_probe` from the
  workspace (`cargo build --release --package sf_core --bin spcs_probe`).
- **Runtime stage** (`debian:bookworm-slim`): just the `spcs_probe` binary +
  `ca-certificates`. `ENTRYPOINT` is the probe.

## Directory layout

```
spcs/
├── Dockerfile   # multi-stage; build context is the repo root
├── build.sh     # builds linux/arm64 (SPCS pool family GEN_ARM_G1_2)
└── README.md
```

## Building

```bash
./tests/docker/spcs/build.sh
```

The build context is the repository root because the image compiles `sf_core`.

## Pushing to a Snowflake image repository

The repository (`testing_setup.public.ud_test_image_repo`) and compute pool
(`ud_test_spcs_pool`) are created by `ci/ci_account_setup/ci_account_setup.sql`.

```bash
snow spcs image-registry login
docker tag <local-tag> <registry>/testing_setup/public/ud_test_image_repo/spcs_probe:1
docker push <registry>/testing_setup/public/ud_test_image_repo/spcs_probe:1
```

`<registry>` is `<org>-<account>.registry.snowflakecomputing.com`.

## Running the e2e test

The orchestrating test lives at
`sf_core/tests/e2e/authentication/spcs_token.rs` and is gated behind the
`auth_spcs_e2e` feature + `#[ignore]`:

```bash
PARAMETER_PATH=parameters.json \
SPCS_PROBE_IMAGE=/testing_setup/public/ud_test_image_repo/spcs_probe:1 \
  cargo test --package sf_core --test e2e_tests --features auth_spcs_e2e -- --ignored spcs_
```
