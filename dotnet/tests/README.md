# .NET Tests

Three test projects, one shared infrastructure library:

| Project | Purpose | CI Blocking? |
|---------|---------|--------------|
| `Snowflake.Data.Tests` | New wrapper integration tests (new driver only) | Yes |
| `Snowflake.Data.Tests.Reference` | Migrated old-driver test suite (runs against both old and new driver) | Old driver: Yes. New driver: No |
| `Snowflake.Data.Tests.Common` | Shared test infrastructure (not a test project itself) | N/A |

## Architecture

```
Snowflake.Data.Tests.Common        (class library)

Snowflake.Data.Tests               (test project)
    References Common. Contains new wrapper tests.
    Trait: Category=E2E

Snowflake.Data.Tests.Reference     (test project)
    References Common. Contains tests migrated from the old driver repo.
    Traits: Category=E2E, Driver=Reference
    Base class: ReferenceTestBase
```

## Running locally

```bash
# New wrapper tests (requires sf_core native lib)
dotnet test tests/Snowflake.Data.Tests --framework net10.0

# Reference tests against old driver (no native lib needed)
SNOWFLAKE_DOTNET_USE_OLD_DRIVER=1 dotnet test tests/Snowflake.Data.Tests.Reference --framework net10.0

# Reference tests against new driver (requires sf_core native lib)
dotnet test tests/Snowflake.Data.Tests.Reference --framework net10.0
```

## CI Jobs (test-dotnet.yml)

- `dotnet_tests` - matrix of TFMs/platforms, runs `Snowflake.Data.Tests`
- `dotnet_tests_reference` - old driver run of `Snowflake.Data.Tests.Reference` (blocking)
- `dotnet_tests_reference_new_driver` - new driver run of `Snowflake.Data.Tests.Reference` (non-blocking, `continue-on-error: true`)

The `dotnet-status` gate only checks `dotnet_tests` + `dotnet_tests_reference` (old driver).
The new-driver reference job is informational — it tracks migration progress without blocking PRs.
