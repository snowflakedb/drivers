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
          testTimeout: 30_000,
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
          testTimeout: 30_000,
          hookTimeout: 180_000,
          globalSetup: ['./tests/setup/e2e.ts'],
        },
      },
    ],
  },
});
