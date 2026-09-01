using Snowflake.Data.Tests.Reference.Fixtures;

namespace Snowflake.Data.Tests.Reference;

public sealed class BasicExecuteQueryTest : ReferenceTestBase
{
    public BasicExecuteQueryTest(ReferenceITFixture fixture, ITestOutputHelper output)
        : base(fixture, output) { }

    // Scenario: should execute simple SELECT returning single value
    [SnowflakeFact]
    public void ShouldExecuteSimpleSelectReturningSingleValue()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT 1 AS value" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 1 AS value";
        using var reader = cmd.ExecuteReader();

        // Then the result should contain value 1
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(1, reader.GetInt32(0));
        Assert.False(reader.IsDBNull(0), "Expected non-NULL value");
        Assert.False(reader.Read(), "Expected exactly one row");
    }
}
