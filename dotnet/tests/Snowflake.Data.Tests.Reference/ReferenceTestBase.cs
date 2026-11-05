using Snowflake.Data.Tests.Reference.Fixtures;

namespace Snowflake.Data.Tests.Reference;

//it's fine
#pragma warning disable xUnit1056
public interface IReferenceTest : IClassFixture<ReferenceITFixture>
#pragma warning restore xUnit1056
{
    public ReferenceITFixture Fixture { get; }

    public ITestOutputHelper Output { get; }
}

[Trait("Category", "E2E")]
[Trait("Driver", "Reference")]
public abstract class ReferenceTestBase : IReferenceTest
{
    public ReferenceTestBase(ReferenceITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    public ReferenceITFixture Fixture { get; }

    public ITestOutputHelper Output { get; }
}
