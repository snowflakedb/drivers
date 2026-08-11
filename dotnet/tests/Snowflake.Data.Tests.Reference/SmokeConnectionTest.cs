using Snowflake.Data.Tests.Reference.Fixtures;

namespace Snowflake.Data.Tests.Reference;

// TODO this test will be replaced soon.
public sealed class SmokeConnectionTest : ReferenceTestBase
{
    public SmokeConnectionTest(RegressionITFixture fixture, ITestOutputHelper output)
        : base(fixture, output) { }

    [SnowflakeFact]
    public void ShouldConnectAndExecuteSelectOne()
    {
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 1";
        var reader = cmd.ExecuteReader();
        reader.Read();
        var result = reader.GetValue(0);

        Assert.Equal(1, Convert.ToInt32(result));
    }
}
