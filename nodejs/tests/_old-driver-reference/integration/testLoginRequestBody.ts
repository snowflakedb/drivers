/* oxlint-disable unicorn/no-empty-file */
// The CLIENT_ENVIRONMENT tests that used to live here moved to
// nodejs/tests/e2e/login-request-body.test.ts.
// The "contains PLATFORM field with mocked lambda env" case is not ported:
// the new driver's login request has no PLATFORM field (BD#70).
// Similar coverage exists in sf_core:
// - detects_is_aws_lambda_from_env in sf_core/src/telemetry/platform_detection/tests.rs
// - sf_core/tests/integration/telemetry/platform_detection.rs
