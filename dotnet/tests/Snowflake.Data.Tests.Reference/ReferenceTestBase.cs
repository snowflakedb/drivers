using Snowflake.Data.Tests.Reference.Fixtures;

namespace Snowflake.Data.Tests.Reference;

[Trait("Category", "E2E")]
[Trait("Driver", "Reference")]
public abstract class ReferenceTestBase : IClassFixture<RegressionITFixture>
{
    protected RegressionITFixture Fixture { get; }

    protected ITestOutputHelper Output { get; }

    protected ReferenceTestBase(RegressionITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }
}
