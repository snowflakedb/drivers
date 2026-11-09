import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    // Two extra attempts after the first failure (3 total). A pass on retry is
    // reported as flaky so intermittent tests are visible without failing CI.
    retry: 2,
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
      {
        // Runs only inside the external-browser container.
        // - Why: the auth flows need its Chromium and Playwright helpers.
        // - Run by: tests/auth/run_auth_browser.sh and run_auth_browser_local.sh.
        // - The scripts set the env and filter by tag.
        // TODO(SNOW-3996212): fail the wrapper when this skip leaves zero tests execute
        name: 'requires_auth_test_container',
        skip: process.env.SF_TEST_HEADLESS_BROWSER !== 'true',
      },
      {
        // Runs only inside a WIF VM.
        // - Why: attestation uses the cloud identity attached to the VM.
        // - Run by: tests/auth/run_wif.sh and run_wif_local.sh.
        // - The scripts set the env and filter by tag.
        name: 'requires_wif_vm',
        skip: process.env.SNOWFLAKE_RUNNING_INSIDE_WIF_VM !== 'true',
      },
      {
        // tests/auth/auth_no_mfa_nodejs.sh selects parameters_aws_local.json.
        name: 'requires_no_mfa',
        skip: process.env.SF_TEST_NO_MFA !== 'true',
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
          name: { label: 'types', color: 'blue' },
          // Vitest type-checks every file in tsconfig.json, not only `include`.
          // So this project also catches type errors in `src/` and in e2e tests.
          typecheck: {
            enabled: true,
            only: true,
            include: ['tests/types/**/*.test-d.ts'],
          },
          globalSetup: ['./tests/setup/types.ts'],
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
