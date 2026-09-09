using Snowflake.Data.Tests.Reference.Fixtures;

namespace Snowflake.Data.Tests.Reference;

[Trait("Driver", "Reference")]
public class BooleanTest : Snowflake.Data.Tests.BooleanTest, IReferenceTest
{
    public BooleanTest(ReferenceITFixture fixture, ITestOutputHelper output) : base(fixture, output)
    {
        Fixture = fixture;
    }

    public new ReferenceITFixture Fixture { get; }
    public new ITestOutputHelper Output => base.Output;
}
