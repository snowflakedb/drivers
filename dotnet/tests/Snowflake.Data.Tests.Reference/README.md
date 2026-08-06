# Snowflake.Data.Tests.Reference

Tests migrated from the old Snowflake .NET driver's integration test suite.

## Purpose

Validates behavioral parity between the old and new driver implementations.
Each test runs against both drivers in CI:

- **Old driver** (blocking): proves the test itself is correct
- **New driver** (non-blocking): tracks migration progress

## Adding migrated tests

1. Create a test class extending `ReferenceTestBase`
2. Use `[SnowflakeFact]` / `[SnowflakeTheory]` attributes
3. Use `TestConnectionFactory.Create(Output)` — it switches driver based on env var
4. Adapt the test to use `DbConnection`/`DbCommand` abstractions (not driver-specific types)

```csharp
namespace Snowflake.Data.Tests.Reference;

public sealed class MyMigratedTest : ReferenceTestBase
{
    public MyMigratedTest(ITFixture fixture, ITestOutputHelper output)
        : base(fixture, output) { }

    [SnowflakeFact]
    public void ShouldDoSomething()
    {
        using var connection = TestConnectionFactory.Create(Output);
        connection.Open();
        // ...
    }
}
```

## Traits

All tests in this project inherit from `ReferenceTestBase` which carries:
- `[Trait("Category", "E2E")]`
- `[Trait("Driver", "Reference")]`

Filter locally: `dotnet test --filter "Driver!=Reference"` to exclude these from a full-solution run.

## Completion criteria

When all reference tests pass on the new driver, promote the `dotnet_tests_reference_new_driver` CI job
to blocking and remove `continue-on-error: true`.
