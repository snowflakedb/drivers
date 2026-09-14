using Snowflake.Data.Tests.Reference.Fixtures;

namespace Snowflake.Data.Tests.Reference;

[Trait("Driver", "Reference")]
public class GeographyTest : Snowflake.Data.Tests.GeographyTest, IReferenceTest
{
    public GeographyTest(ReferenceITFixture fixture, ITestOutputHelper output) : base(fixture, output)
    {
        Fixture = fixture;
    }

    public new ReferenceITFixture Fixture { get; }
    public new ITestOutputHelper Output => base.Output;
}
