using Microsoft.Extensions.Logging;
using NSubstitute;
using NSubstitute.ExceptionExtensions;
using Snowflake.Data.Client;
using Snowflake.Data.Core;
using Snowflake.Data.Core.Errors;
using Snowflake.Data.Interfaces;

namespace Snowflake.Data.Tests;

[Trait("Category", "Unit")]
public sealed class SnowflakeDbDataReaderTest
{
    private const string PublicExceptionSkipReason =
        "Does not work yet: every SnowflakeDbException constructor throws NotImplementedException (TODO next milestone), so no test can observe the SnowflakeDbException this path throws.";

    private const string QueryId = "01bf4a3e-0001-2c4f-0000-00a1b2c3d4e5";

    private static readonly Guid SomeGuid = Guid.Parse("4b7e2c0e-8f3a-4d55-9b1e-2a6c7d8e9f01");
    private static readonly DateTime SomeDate = new(2024, 1, 15, 0, 0, 0, DateTimeKind.Utc);

    private readonly ISnowflakeResultSet _resultSet = Substitute.For<ISnowflakeResultSet>();
    private readonly RecordingLogger _logger = new();
    private readonly SnowflakeDbDataReader _reader;

    public SnowflakeDbDataReaderTest()
    {
        _resultSet.QueryId.Returns(QueryId);
        _reader = new SnowflakeDbDataReader(_resultSet, _logger);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void PropertiesComeFromTheResultSet()
    {
        _resultSet.FieldCount.Returns(3);
        _resultSet.RecordsAffected.Returns(12);
        _resultSet.GetHasRows().Returns(true);

        Assert.Equal(3, _reader.FieldCount);
        Assert.Equal(12, _reader.RecordsAffected);
        Assert.True(_reader.HasRows);
        Assert.False(_reader.IsClosed);
        Assert.Equal(0, _reader.Depth);
        Assert.False(_reader.NextResult());
        Assert.Equal(QueryId, _reader.GetQueryId());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ReadAndColumnMetadataComeFromTheResultSet()
    {
        _resultSet.Read().Returns(true, false);
        _resultSet.GetName(1).Returns("NAME");
        _resultSet.GetDataTypeName(1).Returns("TEXT");
        _resultSet.GetFieldType(1).Returns(typeof(string));
        _resultSet.GetOrdinal("name").Returns(1);

        Assert.True(_reader.Read());
        Assert.False(_reader.Read());
        Assert.Equal("NAME", _reader.GetName(1));
        Assert.Equal("TEXT", _reader.GetDataTypeName(1));
        Assert.Equal(typeof(string), _reader.GetFieldType(1));
        Assert.Equal(1, _reader.GetOrdinal("name"));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void EveryGetterReadsTheSameColumnOfTheResultSet()
    {
        _resultSet.IsDBNull(2).Returns(true);
        _resultSet.GetValue(2).Returns(7L);
        _resultSet.GetFieldValue<bool>(2).Returns(true);
        _resultSet.GetFieldValue<byte>(2).Returns((byte)7);
        _resultSet.GetFieldValue<char>(2).Returns('x');
        _resultSet.GetFieldValue<DateTime>(2).Returns(SomeDate);
        _resultSet.GetFieldValue<decimal>(2).Returns(7.5m);
        _resultSet.GetFieldValue<double>(2).Returns(7.5d);
        _resultSet.GetFieldValue<float>(2).Returns(7.5f);
        _resultSet.GetFieldValue<Guid>(2).Returns(SomeGuid);
        _resultSet.GetFieldValue<short>(2).Returns((short)7);
        _resultSet.GetFieldValue<int>(2).Returns(7);
        _resultSet.GetFieldValue<long>(2).Returns(7L);
        _resultSet.GetFieldValue<string>(2).Returns("seven");

        Assert.True(_reader.IsDBNull(2));
        Assert.Equal(7L, _reader.GetValue(2));
        Assert.Equal(7L, _reader[2]);
        Assert.True(_reader.GetBoolean(2));
        Assert.Equal((byte)7, _reader.GetByte(2));
        Assert.Equal('x', _reader.GetChar(2));
        Assert.Equal(SomeDate, _reader.GetDateTime(2));
        Assert.Equal(7.5m, _reader.GetDecimal(2));
        Assert.Equal(7.5d, _reader.GetDouble(2));
        Assert.Equal(7.5f, _reader.GetFloat(2));
        Assert.Equal(SomeGuid, _reader.GetGuid(2));
        Assert.Equal((short)7, _reader.GetInt16(2));
        Assert.Equal(7, _reader.GetInt32(2));
        Assert.Equal(7L, _reader.GetInt64(2));
        Assert.Equal("seven", _reader.GetString(2));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TheNameIndexerResolvesTheOrdinalFirst()
    {
        _resultSet.GetOrdinal("ID").Returns(0);
        _resultSet.GetValue(0).Returns(1L);

        Assert.Equal(1L, _reader["ID"]);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void GetValuesCopiesAsManyColumnsAsBothSidesHave()
    {
        _resultSet.FieldCount.Returns(2);
        _resultSet.GetValue(0).Returns(1L);
        _resultSet.GetValue(1).Returns(DBNull.Value);
        var values = new object[3];

        var copied = _reader.GetValues(values);

        Assert.Equal(2, copied);
        Assert.Equal(new object?[] { 1L, DBNull.Value, null }, values);
        Assert.Throws<ArgumentNullException>(() => _reader.GetValues(null!));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CopiesPassTheirArgumentsToTheResultSet()
    {
        var chars = new char[4];
        var bytes = new byte[4];
        _resultSet.GetChars(1, 2, chars, 1, 3).Returns(3L);
        _resultSet.GetBytes(0, 0, bytes, 0, 4).Returns(4L);

        Assert.Equal(3L, _reader.GetChars(1, 2, chars, 1, 3));
        Assert.Equal(4L, _reader.GetBytes(0, 0, bytes, 0, 4));
        Assert.Equal(0L, _reader.GetChars(1, 0, null, 0, 0));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void EmptyCopyAtTheEndOfTheBufferIsAccepted()
    {
        var chars = new char[4];
        var bytes = new byte[4];
        _resultSet.GetChars(1, 0, chars, 4, 0).Returns(0L);
        _resultSet.GetBytes(1, 0, bytes, 4, 0).Returns(0L);

        Assert.Equal(0L, _reader.GetChars(1, 0, chars, 4, 0));
        Assert.Equal(0L, _reader.GetBytes(1, 0, bytes, 4, 0));
        _resultSet.Received(1).GetChars(1, 0, chars, 4, 0);
        _resultSet.Received(1).GetBytes(1, 0, bytes, 4, 0);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void InvalidArgumentsAreRejectedBeforeTheResultSetIsReached()
    {
        var buffer = new char[4];

        Assert.Throws<ArgumentOutOfRangeException>(() => _reader.GetChars(1, -1, buffer, 0, 1));
        Assert.Throws<ArgumentOutOfRangeException>(() => _reader.GetChars(1, 0, buffer, 5, 0));
        Assert.Throws<ArgumentOutOfRangeException>(() => _reader.GetChars(1, 0, buffer, 5, 1));
        Assert.Throws<ArgumentOutOfRangeException>(() => _reader.GetChars(1, 0, buffer, 2, 3));
        Assert.Throws<ArgumentOutOfRangeException>(() => _reader.GetBytes(0, 0, new byte[1], -1, 1));
        Assert.Throws<ArgumentNullException>(() => _reader.GetOrdinal(null!));

        Assert.Empty(_resultSet.ReceivedCalls());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task ReadAsyncWithinABatchCompletesSynchronouslyWithoutFetchingAsync()
    {
        _resultSet.TryAdvanceWithinBatch().Returns(true);

        var read = _reader.ReadAsync(CancellationToken.None);

        Assert.Equal(TaskStatus.RanToCompletion, read.Status);
        Assert.True(await read.ConfigureAwait(false));
        _ = _resultSet.DidNotReceiveWithAnyArgs().ReadNextBatchAsync(default);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task ReadAsyncAwaitsAFetchWithTheCallersTokenAsync()
    {
        using var cancellation = new CancellationTokenSource();
        var fetch = new TaskCompletionSource<bool>();
        _resultSet.ReadNextBatchAsync(cancellation.Token).Returns(fetch.Task);

        var read = _reader.ReadAsync(cancellation.Token);
        Assert.False(read.IsCompleted);
        fetch.SetResult(true);

        Assert.True(await read.ConfigureAwait(false));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ReadAsyncWithACancelledTokenIsCancelledAndDoesNotReachTheResultSet()
    {
        using var cancellation = new CancellationTokenSource();
        cancellation.Cancel();

        var read = _reader.ReadAsync(cancellation.Token);

        Assert.True(read.IsCanceled);
        Assert.Empty(_resultSet.ReceivedCalls());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CloseClosesTheResultSetOnceAndDisposeCloses()
    {
        Assert.False(_reader.IsClosed);
        _reader.Close();
        _resultSet.IsClosed.Returns(true);
        _reader.Close();
        _reader.Dispose();

        _resultSet.Received(1).Close();
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TheEnumeratorWalksTheRowsWithoutClosing()
    {
        _resultSet.Read().Returns(true, true, false);
        _resultSet.FieldCount.Returns(1);
        _resultSet.GetName(0).Returns("ID");
        _resultSet.GetFieldType(0).Returns(typeof(long));
        _resultSet.GetValue(0).Returns(1L, 2L);

        var ids = _reader.Cast<IDataRecord>().Select(record => record.GetInt64(0)).ToList();

        Assert.Equal([1L, 2L], ids);
        _resultSet.DidNotReceive().Close();
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void SnowflakeSpecificGettersReadTheColumnOfTheResultSet()
    {
        var time = TimeSpan.FromHours(13);
        var structure = new object();
        int[] array = [1, 2];
        var map = new Dictionary<string, int> { ["a"] = 1 };
        _resultSet.GetFieldValue<TimeSpan>(2).Returns(time);
        _resultSet.GetFieldValue<object>(2).Returns(structure);
        _resultSet.GetFieldValue<int[]>(2).Returns(array);
        _resultSet.GetFieldValue<Dictionary<string, int>>(2).Returns(map);

        Assert.Equal(time, _reader.GetTimeSpan(2));
        Assert.Same(structure, _reader.GetObject<object>(2));
        Assert.Same(array, _reader.GetArray<int>(2));
        Assert.Same(map, _reader.GetMap<string, int>(2));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TheSchemaTableIsNotImplemented()
    {
        Assert.Throws<NotImplementedException>(() => _reader.GetSchemaTable());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = PublicExceptionSkipReason)]
    public void ACarrierFromAGetterIsConvertedToThePublicException()
    {
        _resultSet.GetFieldValue<int>(0).Throws(new SnowflakeDriverException(SFError.INVALID_DATA_CONVERSION, null, "[value hidden]", "FIXED", "Int32"));

        Assert.Equal(270003, Assert.Throws<SnowflakeDbException>(() => _reader.GetInt32(0)).ErrorCode);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = PublicExceptionSkipReason)]
    public void ACarrierFromReadIsConvertedToThePublicException()
    {
        _resultSet.Read().Throws(new SnowflakeDriverException(SFError.QUERY_RESULT_FETCH_FAILED, null, "Failed to read chunk data", 22));

        Assert.Equal(270075, Assert.Throws<SnowflakeDbException>(() => _reader.Read()).ErrorCode);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = PublicExceptionSkipReason)]
    public async Task ACarrierFromAnAwaitedFetchIsConvertedToThePublicExceptionAsync()
    {
        _resultSet.ReadNextBatchAsync(Arg.Any<CancellationToken>())
            .Returns(Task.FromException<bool>(new SnowflakeDriverException(SFError.QUERY_RESULT_FETCH_FAILED, null, "Failed to read chunk data", 22)));

        var exception = await Assert.ThrowsAsync<SnowflakeDbException>(() => _reader.ReadAsync(CancellationToken.None)).ConfigureAwait(false);

        Assert.Equal(270075, exception.ErrorCode);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task CarriersDoNotEscapeThePublicBoundaryAsync()
    {
        var carrier = new SnowflakeDriverException(SFError.DATA_READER_ALREADY_CLOSED, null);
        _resultSet.Read().Throws(carrier);
        _resultSet.GetValue(0).Throws(carrier);
        _resultSet.GetOrdinal("ID").Throws(carrier);
        _resultSet.TryAdvanceWithinBatch().Returns(_ => throw carrier, _ => false);
        _resultSet.ReadNextBatchAsync(Arg.Any<CancellationToken>()).Returns(Task.FromException<bool>(carrier));

        AssertNotACarrier(Record.Exception(() => _reader.Read()));
        AssertNotACarrier(Record.Exception(() => _reader.GetValue(0)));
        AssertNotACarrier(Record.Exception(() => _reader.GetOrdinal("ID")));
        AssertNotACarrier(Record.Exception(() => { _ = _reader.ReadAsync(CancellationToken.None); }));
        AssertNotACarrier(await Record.ExceptionAsync(() => _reader.ReadAsync(CancellationToken.None)).ConfigureAwait(false));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CloseAndDisposeAreLoggedAtDebugLevelWithTheQueryId()
    {
        _reader.Dispose();

        Assert.Equal(
            [
                (LogLevel.Debug, $"Disposing the reader of query {QueryId}", null),
                (LogLevel.Debug, $"Closing the reader of query {QueryId}", null),
            ],
            _logger.Entries);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task ACarrierIsLoggedAtDebugLevelWithTheMethodAndTheQueryIdAsync()
    {
        var carrier = new SnowflakeDriverException(SFError.DATA_READER_ALREADY_CLOSED, null);
        _resultSet.GetFieldValue<int>(0).Throws(carrier);
        _resultSet.Read().Throws(carrier);
        _resultSet.ReadNextBatchAsync(Arg.Any<CancellationToken>()).Returns(Task.FromException<bool>(carrier));

        _ = Record.Exception(() => _reader.GetInt32(0));
        _ = Record.Exception(() => _reader.Read());
        _ = await Record.ExceptionAsync(() => _reader.ReadAsync(CancellationToken.None)).ConfigureAwait(false);

        Assert.Equal(
            [
                (LogLevel.Debug, $"The reader GetInt32 failed for query {QueryId}.", carrier),
                (LogLevel.Debug, $"The reader Read failed for query {QueryId}.", carrier),
                (LogLevel.Debug, $"The reader ReadAsync failed for query {QueryId}.", carrier),
            ],
            _logger.Entries);
    }

    private static void AssertNotACarrier(Exception? exception)
    {
        // SnowflakeDbException construction throws NotImplementedException until SNOW-4382591, so the
        // boundary shows as that exception; the carrier itself must never reach the caller.
        Assert.IsType<NotImplementedException>(exception);
    }

    private sealed class RecordingLogger : ILogger<SnowflakeDbDataReader>
    {
        public List<(LogLevel Level, string Message, Exception? Exception)> Entries { get; } = [];

        public IDisposable? BeginScope<TState>(TState state)
            where TState : notnull => null;

        public bool IsEnabled(LogLevel logLevel) => true;

        public void Log<TState>(LogLevel logLevel, EventId eventId, TState state, Exception? exception, Func<TState, Exception?, string> formatter) =>
            Entries.Add((logLevel, formatter(state, exception), exception));
    }
}
