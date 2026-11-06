using Snowflake.Data.Tests.Reference.Fixtures;

namespace Snowflake.Data.Tests.Reference;

[Trait("Driver", "Reference")]
public class ConnectionTest : Snowflake.Data.Tests.ConnectionTest, IReferenceTest
{
    public ConnectionTest(ReferenceITFixture fixture, ITestOutputHelper output) : base(output, fixture)
    {
        Fixture = fixture;
        Output = output;
    }

    public ReferenceITFixture Fixture { get; }
    public ITestOutputHelper Output { get; }
}
