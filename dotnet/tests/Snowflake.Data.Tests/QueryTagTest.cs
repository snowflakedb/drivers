namespace Snowflake.Data.Tests;

[Trait("Category", "E2E")]
public class QueryTagTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    public QueryTagTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should tag queries when QUERY_TAG is set at connection level
    [SnowflakeFact]
    public void ShouldTagQueriesWhenQueryTagIsSetAtConnectionLevel()
    {
        // Given Snowflake client is logged in with connection option QUERY_TAG set to "conn_tag_e2e"
        using var connection = Fixture.Factory.Create(Output);
        connection.ConnectionString += ";query_tag=conn_tag_e2e";
        connection.Open();

        // When Query "SELECT CURRENT_QUERY_TAG()" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT CURRENT_QUERY_TAG()";
        using var reader = cmd.ExecuteReader();

        // Then the result should contain value "conn_tag_e2e"
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal("conn_tag_e2e", reader.GetString(0));
    }
}
