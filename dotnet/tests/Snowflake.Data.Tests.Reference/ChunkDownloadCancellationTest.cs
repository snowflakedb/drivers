using System.Data.Common;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.Reference.Fixtures;
using NewSnowflakeDbException = Snowflake.Data.SnowflakeDbException;
using OldSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.Reference;

[CollectionDefinition(nameof(ChunkDownloadCancellationTestCollection), DisableParallelization = true)]
public sealed class ChunkDownloadCancellationTestCollection
{
}

[Collection(nameof(ChunkDownloadCancellationTestCollection))]
public sealed class ChunkDownloadCancellationTest : ReferenceTestBase, IDisposable
{
    private const int QueryCancelledErrorCode = 270005;

    private readonly List<string> _createdTables = new();

    public ChunkDownloadCancellationTest(ReferenceITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    public void Dispose()
    {
        if (_createdTables.Count == 0)
            return;

        using var conn = Fixture.Factory.Create(Output);
        conn.Open();
        foreach (var tableName in _createdTables)
        {
            using var command = CreateCommand(conn, $"drop table if exists {tableName}");
            command.ExecuteNonQuery();
        }
    }

    [SnowflakeFact(retriesCount: RetriesCount.Thrice)]
    public async Task ShouldThrowWhenCancelledDuringJsonChunkDownloadAsync()
    {
        // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
        Skip.FutureMilestone();

        const int testRowCount = 90000;
        var ct = TestContext.Current.CancellationToken;
        var tableName = NewTableName("DOTNET_CHUNK_CANCEL_JSON");
        using var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(ct).ConfigureAwait(false);
        SetResultFormat(conn, "JSON");
        await CreateOrReplaceTableAsync(conn, tableName, ct).ConfigureAwait(false);
        await InsertGeneratedRowsAsync(conn, tableName, testRowCount, ct).ConfigureAwait(false);

        using var cmd = CreateCommand(conn, $"select * from {tableName}");
        using var cts = new CancellationTokenSource();

        await Assert.ThrowsAnyAsync<OperationCanceledException>(async () =>
        {
            using var reader = await cmd.ExecuteReaderAsync(cts.Token).ConfigureAwait(false);
            cts.CancelAfter(TimeSpan.FromMilliseconds(100));
            while (await reader.ReadAsync(cts.Token).ConfigureAwait(false))
                _ = reader.GetString(0);
        }).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldThrowWhenCancelledDuringArrowChunkDownloadAsync()
    {
        Skip.FutureMilestone("TODO SNOW-4483010 next milestone");
        const int testRowCount = 100000;
        var ct = TestContext.Current.CancellationToken;
        var tableName = NewTableName("DOTNET_CHUNK_CANCEL_ARROW");
        using var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(ct).ConfigureAwait(false);
        SetResultFormat(conn, "ARROW");
        await CreateOrReplaceTableAsync(conn, tableName, ct).ConfigureAwait(false);
        await InsertGeneratedRowsAsync(conn, tableName, testRowCount, ct).ConfigureAwait(false);

        using var cmd = CreateCommand(conn, $"select * from {tableName}");
        using var cts = new CancellationTokenSource();

        await Assert.ThrowsAnyAsync<OperationCanceledException>(async () =>
        {
            using var reader = await cmd.ExecuteReaderAsync(cts.Token).ConfigureAwait(false);
            cts.CancelAfter(TimeSpan.FromMilliseconds(100));
            while (await reader.ReadAsync(cts.Token).ConfigureAwait(false))
            {
                _ = reader.GetString(0);
                await Task.Yield();
            }
        }).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldThrowBeforeChunkDownloadWhenTokenIsAlreadyCancelledAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException constructors throw NotImplementedException
        Skip.FutureMilestone();
        // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
        Skip.FutureMilestone();

        const int testRowCount = 20000;
        var ct = TestContext.Current.CancellationToken;
        var tableName = NewTableName("DOTNET_CHUNK_CANCEL_PRE");
        using var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(ct).ConfigureAwait(false);
        SetResultFormat(conn, "JSON");
        await CreateOrReplaceTableAsync(conn, tableName, ct).ConfigureAwait(false);
        await InsertGeneratedRowsAsync(conn, tableName, testRowCount, ct).ConfigureAwait(false);

        using var cmd = CreateCommand(conn, $"select * from {tableName}");
        using var cts = new CancellationTokenSource();
        cts.Cancel();

        var oce = await Assert.ThrowsAsync<OperationCanceledException>(
            () => cmd.ExecuteReaderAsync(cts.Token)).ConfigureAwait(false);
        AssertIsSnowflakeDbException(oce.InnerException);
        AssertQueryCancelledInExceptionChain(oce.InnerException!);
    }

    private string NewTableName(string prefix)
    {
        var tableName = $"{prefix}_{Guid.NewGuid():N}";
        _createdTables.Add(tableName);
        return tableName;
    }

    private static DbCommand CreateCommand(ISnowflakeDbConnection connection, string commandText)
    {
        var command = connection.CreateCommand();
        command.CommandText = commandText;
        return command;
    }

    private static void SetResultFormat(ISnowflakeDbConnection connection, string resultFormat)
    {
        var sessionValue = resultFormat == "ARROW" ? "ARROW_FORCE" : resultFormat;
        using var command = CreateCommand(connection, $"alter session set DOTNET_QUERY_RESULT_FORMAT={sessionValue}");
        command.ExecuteNonQuery();
    }

    private static async Task CreateOrReplaceTableAsync(
        ISnowflakeDbConnection connection,
        string tableName,
        CancellationToken cancellationToken)
    {
        using var command = CreateCommand(connection, $"create or replace table {tableName} (col STRING)");
        await command.ExecuteNonQueryAsync(cancellationToken).ConfigureAwait(false);
    }

    private static async Task InsertGeneratedRowsAsync(
        ISnowflakeDbConnection connection,
        string tableName,
        int rowCount,
        CancellationToken cancellationToken)
    {
        using var command = CreateCommand(
            connection,
            $"insert into {tableName}(select randstr(200, random()) from table(generator(rowcount => {rowCount})))");
        await command.ExecuteNonQueryAsync(cancellationToken).ConfigureAwait(false);
    }

    private static void AssertIsSnowflakeDbException(Exception? exception)
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
            Assert.IsType<OldSnowflakeDbException>(exception);
        else
            Assert.IsType<NewSnowflakeDbException>(exception);
    }

    private static void AssertQueryCancelledInExceptionChain(Exception exception)
    {
        var errorCodes = new List<int>();
        for (var current = exception; current is not null; current = current.InnerException)
        {
            switch (current)
            {
                case OldSnowflakeDbException oldException:
                    errorCodes.Add(oldException.ErrorCode);
                    break;
                case NewSnowflakeDbException newException:
                    errorCodes.Add(newException.ErrorCode);
                    break;
            }
        }

        Assert.Contains(QueryCancelledErrorCode, errorCodes);
    }
}
