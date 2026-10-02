import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    silent: 'passed-only',
    // TODO: coverage will be added later
    // coverage: {
    //   provider: "v8",
    //   reporter: ["text", "html", "lcov"],
    //   include: ["src/**/*.ts"],
    // },
    chaiConfig: {
      truncateThreshold: 0,
    },
    tags: [
      /**
       * Auth tests run only inside the external-browser Docker image
       * (`tests/docker/external-browser/Dockerfile`), which bundles Chromium and the
       * Playwright helpers that drive the OAuth, PAT, and MFA/TOTP flows.
       *
       * Run them with `tests/auth/run_auth_browser_local.sh` (builds the image) or, in
       * CI, `tests/auth/run_auth_browser.sh`. If the next argument is not `universal`
       * or `reference`, mode stays `universal` and that argument is a vitest arg.
       * Paths are relative to `nodejs/` and default to `tests/e2e/authentication/`.
       * The runner always passes `--tags-filter requires_auth_test_container`, so
       * only tagged suites run.
       *
       * - `... nodejs` — new driver, tagged suites
       * - `... nodejs tests/e2e/authentication/oauth.test.ts -t "should authenticate"` — new driver, one test
       * - `... nodejs reference` — old driver, tagged suites
       * - `... nodejs reference tests/e2e/authentication/oauth.test.ts -t "should authenticate"` — old driver, one test
       *
       * Those scripts set `SF_TEST_HEADLESS_BROWSER=true` in the container. A plain
       * `npm run test:e2e` leaves the flag unset, so tagged tests are skipped.
       *
       * TODO(SNOW-3996212): fail the wrapper when this skip leaves zero tests executed (#1786).
       */
      {
        name: 'requires_auth_test_container',
        skip: process.env.SF_TEST_HEADLESS_BROWSER !== 'true',
      },
      {
        // Runs only inside a WIF VM, because attestation needs the host's cloud
        // identity from IMDS. tests/auth/run_wif.sh sets this env in that container.
        name: 'requires_wif_vm',
        skip: process.env.SNOWFLAKE_RUNNING_INSIDE_WIF_VM !== 'true',
      },
    ],
    projects: [
      {
        extends: true,
        test: {
          name: { label: 'unit', color: 'cyan' },
          environment: 'node',
          include: ['tests/unit/**/*.test.ts'],
          testTimeout: 1_000,
          hookTimeout: 180_000,
          globalSetup: ['./tests/setup/unit.ts'],
        },
      },
      {
        extends: true,
        test: {
          name: { label: 'e2e', color: 'magenta' },
          environment: 'node',
          include: ['tests/e2e/**/*.test.ts'],
          // TODO: review timeout value - it's quite high but
          // protexts against flakiness when warehouse is being
          // resumed after a pause
          testTimeout: 120_000,
          hookTimeout: 180_000,
          globalSetup: ['./tests/setup/e2e.ts'],
        },
      },
      {
        extends: true,
        test: {
          name: { label: 'e2e-old-driver', color: 'yellow' },
          env: { SNOWFLAKE_NODEJS_E2E_USE_OLD_DRIVER: '1' },
          environment: 'node',
          include: ['tests/e2e/**/*.test.ts'],
          testTimeout: 120_000,
          hookTimeout: 180_000,
          globalSetup: ['./tests/setup/e2e.ts'],
        },
      },
    ],
  },
});
