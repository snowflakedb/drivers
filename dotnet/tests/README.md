# .NET Tests

Four test projects, two shared infrastructure libraries:

| Project | Purpose | CI |
|---------|---------|----|
| `Snowflake.Data.Tests.Unit` | New-driver unit and architecture tests | `dotnet_tests` (new driver only) |
| `Snowflake.Data.Tests.Interop` | Native interop unit tests against `sf_core_stub` | `dotnet_tests` (new driver only) |
| `Snowflake.Data.Tests.E2E` | Live Snowflake E2E tests | `dotnet_tests_e2e` (old + new driver) |
| `Snowflake.Data.Tests.Wiremock` | HTTP/TLS tests against WireMock | `dotnet_tests_e2e` (old + new driver) |
| `Snowflake.Data.Tests.Common` | Shared test infrastructure (not a test project) | N/A |
| `Snowflake.Data.Tests.E2E.Shared` | Connection factory, fixtures and old-driver facade shared by E2E and Wiremock (not a test project) | N/A |

## Running locally

```bash
# Unit tests (requires sf_core for native handle tests; no Snowflake credentials)
dotnet run --project tests/Snowflake.Data.Tests.Unit --framework net10.0

# Interop tests (builds sf_core_stub; no credentials)
dotnet run --project tests/Snowflake.Data.Tests.Interop --framework net10.0

# E2E against the old driver (no native lib)
SNOWFLAKE_DOTNET_USE_OLD_DRIVER=1 dotnet run --project tests/Snowflake.Data.Tests.E2E --framework net10.0

# E2E against the new driver (requires sf_core)
dotnet run --project tests/Snowflake.Data.Tests.E2E --framework net10.0

# Wiremock against the old / new driver (same env vars as E2E)
SNOWFLAKE_DOTNET_USE_OLD_DRIVER=1 dotnet run --project tests/Snowflake.Data.Tests.Wiremock --framework net10.0
dotnet run --project tests/Snowflake.Data.Tests.Wiremock --framework net10.0
```

Use `dotnet run`, not `dotnet test`. See `.claude/skills/run-dotnet-ud-tests/SKILL.md`.

## CI Jobs (test-dotnet.yml)

- `dotnet_tests` — matrix of TFMs/platforms. Unit cells run `Snowflake.Data.Tests.Unit`. Interop cells run `Snowflake.Data.Tests.Interop`. New driver only. Blocking.
- `dotnet_tests_e2e` — old-driver run of E2E + Wiremock. Blocking.
- `dotnet_tests_e2e_new_driver` — new-driver run of E2E + Wiremock. Blocking.

The `dotnet-status` gate checks `dotnet_tests` + both E2E jobs.
