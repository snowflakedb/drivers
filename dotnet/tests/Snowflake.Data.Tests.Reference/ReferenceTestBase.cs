namespace Snowflake.Data.Tests.Reference;

[Trait("Category", "E2E")]
[Trait("Driver", "Reference")]
public abstract class ReferenceTestBase : IClassFixture<ITFixture>
{
    protected ITFixture Fixture { get; }

    protected ITestOutputHelper Output { get; }

    protected ReferenceTestBase(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }
}
