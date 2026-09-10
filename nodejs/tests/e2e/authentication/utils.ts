/**
 * Auth tests run only inside the external-browser Docker image
 * (`tests/docker/external-browser/Dockerfile`), which bundles Chromium and the
 * Playwright helpers that drive the OAuth, PAT, and MFA/TOTP flows.
 *
 * Run them with `tests/auth/run_auth_browser_local.sh nodejs` (builds the image and
 * runs the suite in the container) or, in CI, `tests/auth/run_auth_browser.sh nodejs`.
 *
 * Those scripts set `SF_TEST_HEADLESS_BROWSER=true` in the container; a plain
 * `npm run test:e2e` leaves the flag unset, so these tests are skipped.
 *
 * TODO(SNOW-3996212): fail the wrapper when this skip leaves zero tests executed (#1786).
 */
export const NOT_IN_AUTH_TEST_CONTAINER = process.env.SF_TEST_HEADLESS_BROWSER !== 'true';
