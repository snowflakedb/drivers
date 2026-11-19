# Snowflake.Data.Tests.E2E

Live Snowflake tests. CI runs this project against both drivers:

- **Old driver** (blocking): `SNOWFLAKE_DOTNET_USE_OLD_DRIVER=1`
- **New driver** (blocking): requires `sf_core`

## Adding tests

1. Create a test class extending `LiveE2ETestBase`, or `IClassFixture<ITFixture>` (the assembly fixture routes connections through `TestConnectionFactory`). HTTP-mocked tests use `E2ETestBase` in `.Tests.Wiremock` with `Category=Wiremock`.
2. Use `[SnowflakeFact]` / `[SnowflakeTheory]`.
3. Open connections through `Fixture.Factory` so old/new switching stays in one place.

```csharp
namespace Snowflake.Data.Tests.E2E;

public sealed class MyTest : LiveE2ETestBase
{
    public MyTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output) { }

    [SnowflakeFact]
    public void ShouldDoSomething()
    {
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();
    }
}
```

HTTP-mocked tests belong in `Snowflake.Data.Tests.Wiremock`.
