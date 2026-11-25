namespace Snowflake.Data.Tests.E2E;

[Trait("Category", "E2E")]
[Trait("Category", "Conversion")]
public class LargeResultSetTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    public LargeResultSetTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should process one million row result set
    [SnowflakeFact]
    public async Task ShouldProcessOneMillionRowResultSetAsync()
    {
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v ORDER BY id" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v ORDER BY id";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then there are 1000000 numbered sequentially rows returned
        long expected = 0;
        while (await reader.ReadAsync(ct).ConfigureAwait(false))
        {
            Assert.Equal(expected, reader.GetInt64(0));
            expected++;
        }

        Assert.Equal(1_000_000L, expected);
    }

    // Scenario: should process ten thousand string rows when initial chunk is empty
    [SnowflakeFact]
    public async Task ShouldProcessTenThousandStringRowsWhenInitialChunkIsEmptyAsync()
    {
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // When Query "select L_COMMENT from SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM limit 10000" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "select L_COMMENT from SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM limit 10000";
        using var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        // Then there are 10000 rows returned
        var rowCount = 0;
        while (await reader.ReadAsync(ct).ConfigureAwait(false))
        {
            Assert.False(string.IsNullOrEmpty(reader.GetString(0)), $"Expected a non-empty L_COMMENT in row {rowCount}");
            rowCount++;
        }

        Assert.Equal(10_000, rowCount);
    }
}
