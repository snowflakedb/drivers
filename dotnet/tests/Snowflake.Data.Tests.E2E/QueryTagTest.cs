extern alias OldDriver;
using System.Data.Common;
using Snowflake.Data.Tests.Compatibility;
using NewSnowflakeDbCommand = Snowflake.Data.Client.SnowflakeDbCommand;
using OldSnowflakeDbCommand = OldDriver::Snowflake.Data.Client.SnowflakeDbCommand;

namespace Snowflake.Data.Tests.E2E;

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
    public async Task ShouldTagQueriesWhenQueryTagIsSetAtConnectionLevelAsync()
    {
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in with connection option QUERY_TAG set to "conn_tag_e2e"
        using var connection = Fixture.Factory.Create(Output);
        connection.ConnectionString += ";query_tag=conn_tag_e2e";
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Query "SELECT CURRENT_QUERY_TAG()" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT CURRENT_QUERY_TAG()";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then the result should contain value "conn_tag_e2e"
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        Assert.Equal("conn_tag_e2e", reader.GetString(0));
    }

    // Scenario: should tag a single query via statement-level query tag
    [SnowflakeFact]
    public async Task ShouldTagASingleQueryViaStatementLevelQueryTagAsync()
    {
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Query "SELECT CURRENT_QUERY_TAG()" is executed with statement-level QUERY_TAG "stmt_tag_e2e"
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT CURRENT_QUERY_TAG()";
        SetStatementQueryTag(cmd, "stmt_tag_e2e");
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then the result should contain value "stmt_tag_e2e"
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        Assert.Equal("stmt_tag_e2e", reader.GetString(0));
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should not leak statement-level query tag into session state
    [SnowflakeFact]
    public async Task ShouldNotLeakStatementLevelQueryTagIntoSessionStateAsync()
    {
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Query "SELECT CURRENT_QUERY_TAG()" is executed with statement-level QUERY_TAG "stmt_tag_e2e"
        using var taggedCmd = connection.CreateCommand();
        taggedCmd.CommandText = "SELECT CURRENT_QUERY_TAG()";
        SetStatementQueryTag(taggedCmd, "stmt_tag_e2e");
        using (var taggedReader = await taggedCmd.ExecuteReaderAsync(ct).ConfigureAwait(false))
        {
            Assert.True(await taggedReader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
            Assert.Equal("stmt_tag_e2e", taggedReader.GetString(0));
        }

        // And Query "SELECT CURRENT_QUERY_TAG()" is executed without a statement-level tag
        using var untaggedCmd = connection.CreateCommand();
        untaggedCmd.CommandText = "SELECT CURRENT_QUERY_TAG()";
        using var reader = await untaggedCmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then the last result should contain empty value
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        Assert.False(reader.IsDBNull(0), "Expected an empty string, not NULL");
        Assert.Equal(string.Empty, reader.GetString(0));
    }

    // Scenario: should let a statement QUERY_TAG override the connection QUERY_TAG
    [SnowflakeFact]
    public async Task ShouldLetAStatementQueryTagOverrideTheConnectionQueryTagAsync()
    {
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in with connection option QUERY_TAG set to "conn"
        using var connection = Fixture.Factory.Create(Output);
        connection.ConnectionString += ";query_tag=conn";
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Query "SELECT CURRENT_QUERY_TAG()" is executed with statement-level QUERY_TAG "stmt"
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT CURRENT_QUERY_TAG()";
        SetStatementQueryTag(cmd, "stmt");
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then the result should contain value "stmt"
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        Assert.Equal("stmt", reader.GetString(0));
    }

    private static void SetStatementQueryTag(DbCommand command, string queryTag)
    {
        switch (command)
        {
            case OldSnowflakeDbCommand oldCommand:
                oldCommand.QueryTag = queryTag;
                break;
            case NewSnowflakeDbCommand newCommand:
                newCommand.QueryTag = queryTag;
                break;
            default:
                throw new InvalidOperationException($"Unexpected command type {command.GetType()}");
        }
    }
}
