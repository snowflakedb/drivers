using System.Data.Common;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.Reference.Fixtures;

namespace Snowflake.Data.Tests.Reference;

public sealed class SFDbDataReaderGetEnumeratorTest : ReferenceTestBase
{
    public SFDbDataReaderGetEnumeratorTest(ReferenceITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldEnumerateAllRowsAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        var tableName = UniqueTableName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            await CreateAndPopulateTestTableAsync(conn, tableName, TestContext.Current.CancellationToken).ConfigureAwait(false);

            using var reader = await ExecuteReaderAsync(conn, $"select * from {tableName}", TestContext.Current.CancellationToken).ConfigureAwait(false);

            var enumerator = reader.GetEnumerator();
            Assert.True(enumerator.MoveNext());
            Assert.Equal(3, Assert.IsAssignableFrom<DbDataRecord>(enumerator.Current).GetInt64(0));
            Assert.True(enumerator.MoveNext());
            Assert.Equal(5, Assert.IsAssignableFrom<DbDataRecord>(enumerator.Current).GetInt64(0));
            Assert.True(enumerator.MoveNext());
            Assert.Equal(8, Assert.IsAssignableFrom<DbDataRecord>(enumerator.Current).GetInt64(0));
            Assert.False(enumerator.MoveNext());

            reader.Close();
        }
        finally
        {
            await DropTestTableAsync(conn, tableName, TestContext.Current.CancellationToken).ConfigureAwait(false);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReturnEmptyEnumeratorWhenNoRowsAreReturnedAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        var tableName = UniqueTableName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            await CreateAndPopulateTestTableAsync(conn, tableName, TestContext.Current.CancellationToken).ConfigureAwait(false);

            using var reader = await ExecuteReaderAsync(conn, $"select * from {tableName} WHERE cola > 10", TestContext.Current.CancellationToken).ConfigureAwait(false);

            var enumerator = reader.GetEnumerator();
            Assert.False(enumerator.MoveNext());
            Assert.Null(enumerator.Current);

            reader.Close();
        }
        finally
        {
            await DropTestTableAsync(conn, tableName, TestContext.Current.CancellationToken).ConfigureAwait(false);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldCastEnumeratedRowsToDataRecordsAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        var tableName = UniqueTableName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            await CreateAndPopulateTestTableAsync(conn, tableName, TestContext.Current.CancellationToken).ConfigureAwait(false);

            using var reader = await ExecuteReaderAsync(conn, $"select * from {tableName}", TestContext.Current.CancellationToken).ConfigureAwait(false);

            var dataRecords = reader.Cast<DbDataRecord>().ToList();
            Assert.Equal(3, dataRecords.Count);

            reader.Close();
        }
        finally
        {
            await DropTestTableAsync(conn, tableName, TestContext.Current.CancellationToken).ConfigureAwait(false);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldNotEnterForEachLoopWhenResultIsEmptyAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        var tableName = UniqueTableName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            await CreateAndPopulateTestTableAsync(conn, tableName, TestContext.Current.CancellationToken).ConfigureAwait(false);

            using var reader = await ExecuteReaderAsync(conn, $"select * from {tableName} WHERE cola > 10", TestContext.Current.CancellationToken).ConfigureAwait(false);

            foreach (var record in reader)
            {
                Assert.Fail("Should not enter when results is empty");
            }

            await reader.CloseAsync().ConfigureAwait(false);
        }
        finally
        {
            await DropTestTableAsync(conn, tableName, TestContext.Current.CancellationToken).ConfigureAwait(false);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldThrowNotSupportedExceptionWhenEnumeratorIsResetAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        var tableName = UniqueTableName();
        using var conn = await OpenConnectionAsync(resultFormat, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            await CreateAndPopulateTestTableAsync(conn, tableName, TestContext.Current.CancellationToken).ConfigureAwait(false);

            using var reader = await ExecuteReaderAsync(conn, $"select * from {tableName}", TestContext.Current.CancellationToken).ConfigureAwait(false);

            var enumerator = reader.GetEnumerator();
            Assert.True(enumerator.MoveNext());

            Assert.Throws<NotSupportedException>(() => enumerator.Reset());

            reader.Close();
        }
        finally
        {
            await DropTestTableAsync(conn, tableName, TestContext.Current.CancellationToken).ConfigureAwait(false);
        }
    }

    private static string UniqueTableName() => $"DOTNET_TEST_ENUMERATOR_{Guid.NewGuid():N}";

    private async Task<ISnowflakeDbConnection> OpenConnectionAsync(string resultFormat, CancellationToken cancellationToken)
    {
        var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(cancellationToken).ConfigureAwait(false);
        conn.PreventPooling();
        await ExecuteNonQueryAsync(
            conn,
            $"alter session set DOTNET_QUERY_RESULT_FORMAT = {SessionResultFormat(resultFormat)}",
            cancellationToken).ConfigureAwait(false);
        return conn;
    }

    private static string SessionResultFormat(string resultFormat) =>
        resultFormat == "ARROW" ? "ARROW_FORCE" : resultFormat;

    private static async Task CreateAndPopulateTestTableAsync(
        ISnowflakeDbConnection conn,
        string tableName,
        CancellationToken cancellationToken)
    {
        await ExecuteNonQueryAsync(conn, $"CREATE OR REPLACE TABLE {tableName}(cola NUMBER)", cancellationToken).ConfigureAwait(false);
        await ExecuteNonQueryAsync(conn, $"insert into {tableName} values (3),(5),(8)", cancellationToken).ConfigureAwait(false);
    }

    private static async Task DropTestTableAsync(
        ISnowflakeDbConnection conn,
        string tableName,
        CancellationToken cancellationToken)
    {
        var count = await ExecuteNonQueryAsync(conn, $"drop table if exists {tableName}", cancellationToken).ConfigureAwait(false);
        Assert.Equal(0, count);
    }

    private static async Task<int> ExecuteNonQueryAsync(
        ISnowflakeDbConnection conn,
        string commandText,
        CancellationToken cancellationToken)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = commandText;
        return await cmd.ExecuteNonQueryAsync(cancellationToken).ConfigureAwait(false);
    }

    private static async Task<DbDataReader> ExecuteReaderAsync(
        ISnowflakeDbConnection conn,
        string commandText,
        CancellationToken cancellationToken)
    {
        var cmd = conn.CreateCommand();
        cmd.CommandText = commandText;
        return await cmd.ExecuteReaderAsync(cancellationToken).ConfigureAwait(false);
    }
}
