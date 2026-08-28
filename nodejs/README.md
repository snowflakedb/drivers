# snowflake-sdk

Node.js driver for Snowflake.

## Building

The driver is TypeScript (`src/`) on top of a native addon compiled from the Rust crate `nodejs_bridge`. Three scripts in `scripts/` produce the pieces:

- `npm run build:core` — compiles the Rust crate into a native `.node` addon under `_build/snowflake-sdk-core/`.
- `npm run build:sdk` — compiles `src/` into a publish-ready package under `_build/snowflake-sdk/`.
- `npm run build:link-local` — links the built packages into the workspace so code (and `tsc`) resolves them by name, the way a real consumer would. (The native addon is resolved by a platform-specific package name that only exists after the build.)

You rarely run these by hand — tests build what they need automatically (see [Testing](#testing)).

### Generated code: `src/core/binary-types.generated.ts`

This file is the TypeScript API of the native addon. It's produced by `build:core` but **committed to git on purpose**: having it in the source tree lets you typecheck, lint, and get IDE autocomplete without the Rust toolchain or a native build.

Do not edit it by hand. If you change the Rust API, run `npm run build:core` to regenerate it, then commit the result.

## Testing

Tests live in `tests/` and run with [Vitest](https://vitest.dev/). The needed build steps run automatically before each suite (see `tests/setup/`), so just run:

- `npm run test:unit` — fast tests against the TypeScript source in `src/`. Builds only the native core and links it.
- `npm run test:e2e` — end-to-end tests against a real Snowflake account. Builds and links the full SDK, then imports `snowflake-sdk` by name — the same package and addon resolution a real user gets.
- `npm run test:e2e-old-driver` — the same e2e tests run against the old `snowflake-sdk` (v3) driver, so we can compare behavior.

Each run rebuilds instead of caching, so tests always run against current source; Rust's incremental compiler keeps a no-op rebuild cheap.
