using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E.Fixtures;

namespace Snowflake.Data.Tests.E2E;

public sealed class SFDbCommandSlowTest : LiveE2ETestBase
{
    public SFDbCommandSlowTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeFact]
    public async Task ShouldExecuteLongRunningQueryAsync()
    {
        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "select count(seq4()) from table(generator(timelimit => 60)) v order by 1";
        using var reader = cmd.ExecuteReader();
        Assert.True(reader.Read());

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldReturnMinusOneWhenRowsAffectedOverflowsIntAsync()
    {
        Skip.When(true, "This test case takes too much time so run it manually");

        var tableName = $"SFDBCOMMANDSLOWTEST_{Guid.NewGuid():N}";
        using var conn = Fixture.Factory.Create(Output, NonPooledConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            ExecuteNonQuery(conn, $"CREATE OR REPLACE TABLE {tableName}(c1 NUMBER)");

            var affected = ExecuteNonQuery(
                conn,
                $"INSERT INTO {tableName} SELECT SEQ4() FROM TABLE(GENERATOR(ROWCOUNT=>2147484000))");

            Assert.Equal(-1, affected);
        }
        finally
        {
            ExecuteNonQuery(conn, $"DROP TABLE IF EXISTS {tableName}");
            await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        }
    }

    private string NonPooledConnectionString()
    {
        using var probe = Fixture.Factory.Create(Output);
        return $"{probe.ConnectionString.TrimEnd(';')};poolingEnabled=false";
    }

    private static int ExecuteNonQuery(ISnowflakeDbConnection conn, string commandText)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = commandText;
        return cmd.ExecuteNonQuery();
    }
}
