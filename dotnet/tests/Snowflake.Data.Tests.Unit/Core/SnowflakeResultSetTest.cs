using Apache.Arrow;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Logging.Abstractions;
using NSubstitute;
using Snowflake.Data.Core;
using Snowflake.Data.Core.DataConversion.ResultSetConversion;
using Snowflake.Data.Core.Errors;
using Snowflake.Data.Core.ResultSet;
using Snowflake.Data.Core.Session;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Tests.Core;

[Trait("Category", "Unit")]
public sealed class SnowflakeResultSetTest
{
    private const string ConverterSkipReason = "Needs the TEXT and FIXED converters; PR stack #3380 unskips this test.";

    private const string QueryId = "01bf4a3e-0001-2c4f-0000-00a1b2c3d4e5";

    private static readonly ResultSetDescriptor IdAndName = WithQueryId(
        UnitBatches.Descriptor(UnitBatches.FixedColumn("ID"), UnitBatches.TextColumn("NAME")));

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = ConverterSkipReason)]
    public void ReadWalksEveryRowAcrossBatchesAndSkipsEmptyOnes()
    {
        var source = new FakeRecordBatchSource(
            IdNameBatch((1, "a"), (2, "b")),
            IdNameBatch(),
            IdNameBatch((3, "c")));
        using var resultSet = ResultSet(source, IdAndName);

        var rows = new List<(long, string?)>();
        while (resultSet.Read())
            rows.Add((resultSet.GetFieldValue<long>(0), resultSet.GetFieldValue<string>(1)));

        Assert.Equal([(1L, "a"), (2L, "b"), (3L, "c")], rows);
        Assert.False(resultSet.Read());
        Assert.Equal(4, source.NextCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = ConverterSkipReason)]
    public async Task AdvancingWithinABatchFetchesNothingAndReadNextBatchAsyncMovesToTheNextBatchAsync()
    {
        var source = new FakeRecordBatchSource(IdNameBatch((1, "a"), (2, "b")), IdNameBatch(), IdNameBatch((3, "c")));
        using var resultSet = ResultSet(source, IdAndName);

        Assert.False(resultSet.TryAdvanceWithinBatch());
        Assert.True(await resultSet.ReadNextBatchAsync(CancellationToken.None).ConfigureAwait(false));
        Assert.Equal(1L, resultSet.GetFieldValue<long>(0));
        Assert.True(resultSet.TryAdvanceWithinBatch());
        Assert.Equal(2L, resultSet.GetFieldValue<long>(0));
        Assert.Equal(1, source.NextAsyncCalls);
        Assert.False(resultSet.TryAdvanceWithinBatch());
        Assert.True(await resultSet.ReadNextBatchAsync(CancellationToken.None).ConfigureAwait(false));
        Assert.Equal("c", resultSet.GetFieldValue<string>(1));
        Assert.False(resultSet.TryAdvanceWithinBatch());
        Assert.False(await resultSet.ReadNextBatchAsync(CancellationToken.None).ConfigureAwait(false));
        Assert.Equal(4, source.NextAsyncCalls);
        Assert.Equal(0, source.NextCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = ConverterSkipReason)]
    public async Task ACancelledFetchPropagatesWithTheQueryIdAndTheResultStaysUsableAsync()
    {
        var cancellation = new SnowflakeCancellationException(SFError.QUERY_CANCELLED, null, null);
        var source = new FakeRecordBatchSource(IdNameBatch((1, "a"))) { FailNextWith = cancellation };
        using var resultSet = ResultSet(source, IdAndName);

        var exception = await Assert.ThrowsAsync<SnowflakeCancellationException>(() => resultSet.ReadNextBatchAsync(CancellationToken.None)).ConfigureAwait(false);

        Assert.Same(cancellation, exception);
        Assert.Equal(QueryId, exception.QueryId);
        Assert.True(await resultSet.ReadNextBatchAsync(CancellationToken.None).ConfigureAwait(false));
        Assert.Equal(1L, resultSet.GetFieldValue<long>(0));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void AFetchFailurePropagatesUnchangedWithTheQueryId()
    {
        var failure = new SnowflakeDriverException(SFError.QUERY_RESULT_FETCH_FAILED, null, "Failed to read chunk data", 22);
        using var resultSet = ResultSet(new FakeRecordBatchSource { FailNextWith = failure }, IdAndName);

        var exception = Assert.Throws<SnowflakeDriverException>(() => resultSet.Read());

        Assert.Same(failure, exception);
        Assert.Equal(QueryId, exception.QueryId);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ValuesBeforeTheFirstReadAndAfterTheLastRowAreNoCurrentRow()
    {
        using var resultSet = ResultSet(new FakeRecordBatchSource(IdNameBatch((1, "a"))), IdAndName);

        AssertDriverError(SFError.DATA_READER_NO_CURRENT_ROW, () => resultSet.GetValue(0));
        AssertDriverError(SFError.DATA_READER_NO_CURRENT_ROW, () => resultSet.IsDBNull(0));
        Assert.True(resultSet.Read());
        Assert.False(resultSet.Read());
        AssertDriverError(SFError.DATA_READER_NO_CURRENT_ROW, () => resultSet.GetFieldValue<long>(0));
        AssertDriverError(SFError.DATA_READER_NO_CURRENT_ROW, () => resultSet.GetChars(1, 0, null, 0, 0));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = ConverterSkipReason)]
    public void ConversionFailuresAreInvalidDataConversionWithTheQueryId()
    {
        var descriptor = WithQueryId(UnitBatches.Descriptor(UnitBatches.FixedColumn("BIG"), UnitBatches.FixedColumn("NOTHING"), UnitBatches.TextColumn("WORD")));
        var batch = UnitBatches.Batch(UnitBatches.Int64(3_000_000_000), UnitBatches.Int64([null]), UnitBatches.Strings("abc"));
        using var resultSet = ResultSet(new FakeRecordBatchSource(batch), descriptor);
        resultSet.Read();

        AssertDriverError(SFError.INVALID_DATA_CONVERSION, () => resultSet.GetFieldValue<int>(0));
        AssertDriverError(SFError.INVALID_DATA_CONVERSION, () => resultSet.GetFieldValue<string>(1));
        AssertDriverError(SFError.INVALID_DATA_CONVERSION, () => resultSet.GetFieldValue<long>(2));
        AssertDriverError(SFError.INVALID_DATA_CONVERSION, () => resultSet.GetFieldValue<bool>(2));
        AssertDriverError(SFError.INVALID_DATA_CONVERSION, () => resultSet.GetBytes(2, 0, null, 0, 0));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = ConverterSkipReason)]
    public void CharsAreCopiedFromTheCurrentRow()
    {
        using var resultSet = ResultSet(new FakeRecordBatchSource(IdNameBatch((1, "abc"))), IdAndName);
        var buffer = new char[4];
        resultSet.Read();

        Assert.Equal(3, resultSet.GetChars(1, 0, buffer, 1, 3));
        Assert.Equal(['\0', 'a', 'b', 'c'], buffer);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void AnOrdinalOutOfRangeIsColumnIndexOutOfBound()
    {
        using var resultSet = ResultSet(new FakeRecordBatchSource(IdNameBatch((1, "a"))), IdAndName);
        resultSet.Read();

        AssertDriverError(SFError.COLUMN_INDEX_OUT_OF_BOUND, () => resultSet.GetValue(2));
        AssertDriverError(SFError.COLUMN_INDEX_OUT_OF_BOUND, () => resultSet.GetValue(-1));
        AssertDriverError(SFError.COLUMN_INDEX_OUT_OF_BOUND, () => resultSet.GetName(2));
        AssertDriverError(SFError.COLUMN_INDEX_OUT_OF_BOUND, () => resultSet.GetDataTypeName(2));
        AssertDriverError(SFError.COLUMN_INDEX_OUT_OF_BOUND, () => resultSet.GetFieldType(2));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task ReadingAfterCloseIsAlreadyClosedAsync()
    {
        var resultSet = ResultSet(new FakeRecordBatchSource(IdNameBatch((1, "a"))), IdAndName);
        resultSet.Read();
        resultSet.Close();

        AssertDriverError(SFError.DATA_READER_ALREADY_CLOSED, () => resultSet.Read());
        AssertDriverError(SFError.DATA_READER_ALREADY_CLOSED, () => resultSet.TryAdvanceWithinBatch());
        var closed = await Assert.ThrowsAsync<SnowflakeDriverException>(() => resultSet.ReadNextBatchAsync(CancellationToken.None)).ConfigureAwait(false);
        Assert.Equal(SFError.DATA_READER_ALREADY_CLOSED, closed.Error);
        AssertDriverError(SFError.DATA_READER_ALREADY_CLOSED, () => resultSet.GetValue(0));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = ConverterSkipReason)]
    public void MetadataStaysAvailableAfterClose()
    {
        var resultSet = ResultSet(new FakeRecordBatchSource(), IdAndName);

        resultSet.Close();

        Assert.True(resultSet.IsClosed);
        Assert.False(resultSet.GetHasRows());
        Assert.Equal(2, resultSet.FieldCount);
        Assert.Equal("NAME", resultSet.GetName(1));
        Assert.Equal("FIXED", resultSet.GetDataTypeName(0));
        Assert.Equal(typeof(string), resultSet.GetFieldType(1));
        Assert.Equal(1, resultSet.GetOrdinal("name"));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CloseAndDisposeReleaseTheSourceOnce()
    {
        var source = new FakeRecordBatchSource(IdNameBatch((1, "a")));
        var resultSet = ResultSet(source, IdAndName);
        resultSet.Read();

        resultSet.Close();
        resultSet.Close();
        resultSet.Dispose();

        Assert.Equal(1, source.DisposeCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void HasRowsUsesTheReportedRowCountWithoutFetching()
    {
        var withRows = UnitBatches.Descriptor(UnitBatches.FixedColumn());
        withRows.RowCount = 3;
        var withoutRows = UnitBatches.Descriptor(UnitBatches.FixedColumn());
        withoutRows.RowCount = 0;
        var source = new FakeRecordBatchSource();
        using var resultSetWithRows = ResultSet(source, withRows);
        using var resultSetWithoutRows = ResultSet(source, withoutRows);

        Assert.True(resultSetWithRows.GetHasRows());
        Assert.False(resultSetWithoutRows.GetHasRows());
        Assert.Equal(0, source.NextCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = ConverterSkipReason)]
    public void HasRowsWithoutARowCountPeeksTheFirstRowsAndReadStillReturnsThem()
    {
        var source = new FakeRecordBatchSource(IdNameBatch(), IdNameBatch((7, "x")));
        using var resultSet = ResultSet(source, IdAndName);

        Assert.True(resultSet.GetHasRows());
        Assert.True(resultSet.GetHasRows());
        Assert.Equal(2, source.NextCalls);
        Assert.True(resultSet.Read());
        Assert.Equal(7L, resultSet.GetFieldValue<long>(0));
        Assert.False(resultSet.Read());
        Assert.True(resultSet.GetHasRows());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void HasRowsIsFalseForAResultWithoutRows()
    {
        var source = new FakeRecordBatchSource(IdNameBatch());
        using var resultSet = ResultSet(source, IdAndName);

        Assert.False(resultSet.GetHasRows());
        Assert.False(resultSet.Read());
        Assert.Equal(2, source.NextCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void GetOrdinalFindsTheColumnAndAMissingNameIsColumnIndexOutOfBound()
    {
        using var resultSet = ResultSet(new FakeRecordBatchSource(), IdAndName);

        Assert.Equal(1, resultSet.GetOrdinal("name"));
        var missing = AssertDriverError(SFError.COLUMN_INDEX_OUT_OF_BOUND, () => resultSet.GetOrdinal("missing"));
        Assert.Equal("missing", Assert.Single(missing.Args));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = ConverterSkipReason)]
    public void ColumnMetadataComesFromTheDescriptorAndTheConverters()
    {
        var descriptor = UnitBatches.Descriptor(UnitBatches.FixedColumn("ID"), UnitBatches.Column("CREATED", "timestamp_ltz"));
        using var resultSet = ResultSet(new FakeRecordBatchSource(), descriptor);

        Assert.Equal(2, resultSet.FieldCount);
        Assert.Equal("ID", resultSet.GetName(0));
        Assert.Equal("FIXED", resultSet.GetDataTypeName(0));
        Assert.Equal("TIMESTAMP_LTZ", resultSet.GetDataTypeName(1));
        Assert.Equal(typeof(long), resultSet.GetFieldType(0));
        Assert.Throws<NotImplementedException>(() => resultSet.GetFieldType(1));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void EachColumnGetsItsConverterFromTheFactoryOnce()
    {
        var factory = Substitute.For<IColumnConverterFactory>();
        factory.Create(Arg.Any<ColumnMetadata>()).Returns(call => new NotImplementedColumnConverter(call.Arg<ColumnMetadata>().Type));

        using var resultSet = new SnowflakeResultSet(ResultSetInterpreter.Instance, new FakeRecordBatchSource(), factory, IdAndName, -1, NullLogger<SnowflakeResultSet>.Instance);

        Received.InOrder(() =>
        {
            factory.Create(IdAndName.Columns[0]);
            factory.Create(IdAndName.Columns[1]);
        });
        Assert.Equal(2, factory.ReceivedCalls().Count());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task ReadNextBatchAsyncStartsAtTheBatchHasRowsPeekedAsync()
    {
        var source = new FakeRecordBatchSource(IdNameBatch(), IdNameBatch((7, "x")));
        using var resultSet = ResultSet(source, IdAndName);

        Assert.True(resultSet.GetHasRows());
        Assert.Equal(2, source.NextCalls);
        Assert.True(await resultSet.ReadNextBatchAsync(CancellationToken.None).ConfigureAwait(false));

        Assert.Equal(0, source.NextAsyncCalls);
        Assert.False(resultSet.TryAdvanceWithinBatch());
        Assert.False(await resultSet.ReadNextBatchAsync(CancellationToken.None).ConfigureAwait(false));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void HasRowsAfterCloseIsFalseWithoutFetchingWhenTheRowCountIsUnknown()
    {
        var source = new FakeRecordBatchSource(IdNameBatch((1, "a")));
        var resultSet = ResultSet(source, IdAndName);
        resultSet.Close();

        Assert.False(resultSet.GetHasRows());
        Assert.Equal(0, source.NextCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void HasRowsAfterCloseIsFalseWhenTheRowCountIsPositive()
    {
        var descriptor = WithQueryId(UnitBatches.Descriptor(UnitBatches.FixedColumn("ID"), UnitBatches.TextColumn("NAME")));
        descriptor.RowCount = 3;
        var resultSet = ResultSet(new FakeRecordBatchSource(), descriptor);
        Assert.True(resultSet.GetHasRows());
        resultSet.Close();

        Assert.False(resultSet.GetHasRows());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task ReadNextBatchAsyncAfterTheEndFetchesNothingMoreAsync()
    {
        var source = new FakeRecordBatchSource(IdNameBatch((1, "a")));
        using var resultSet = ResultSet(source, IdAndName);
        Assert.True(await resultSet.ReadNextBatchAsync(CancellationToken.None).ConfigureAwait(false));
        Assert.False(await resultSet.ReadNextBatchAsync(CancellationToken.None).ConfigureAwait(false));
        var fetchesAtTheEnd = source.NextAsyncCalls;

        Assert.False(await resultSet.ReadNextBatchAsync(CancellationToken.None).ConfigureAwait(false));
        Assert.Equal(fetchesAtTheEnd, source.NextAsyncCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task CarrierThatAlreadyHasAQueryIdKeepsItAsync()
    {
        var source = new FakeRecordBatchSource { FailNextWith = new SnowflakeDriverException(SFError.QUERY_RESULT_FETCH_FAILED, "other-query") };
        using var resultSet = ResultSet(source, IdAndName);

        var syncFailure = Assert.Throws<SnowflakeDriverException>(() => resultSet.Read());
        source.FailNextWith = new SnowflakeDriverException(SFError.QUERY_RESULT_FETCH_FAILED, "other-query");
        var asyncFailure = await Assert.ThrowsAsync<SnowflakeDriverException>(() => resultSet.ReadNextBatchAsync(CancellationToken.None)).ConfigureAwait(false);

        Assert.Equal("other-query", syncFailure.QueryId);
        Assert.Equal("other-query", asyncFailure.QueryId);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void EmptyQueryIdLeavesCarriersWithoutOne()
    {
        var descriptor = UnitBatches.Descriptor(UnitBatches.FixedColumn("ID"), UnitBatches.TextColumn("NAME"));
        descriptor.QueryId = string.Empty;
        var resultSet = ResultSet(new FakeRecordBatchSource(), descriptor);
        resultSet.Close();

        var failure = Assert.Throws<SnowflakeDriverException>(() => resultSet.Read());

        Assert.Equal(SFError.DATA_READER_ALREADY_CLOSED, failure.Error);
        Assert.Null(failure.QueryId);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CancelledFetchIsLoggedAtDebugAndFailedFetchAtErrorWithTheCarrier()
    {
        var logger = Substitute.For<ILogger<SnowflakeResultSet>>();
        var source = new FakeRecordBatchSource { FailNextWith = new SnowflakeCancellationException(SFError.QUERY_CANCELLED, null, null) };
        using var resultSet = new SnowflakeResultSet(ResultSetInterpreter.Instance, source, ColumnConverterFactory.Instance, IdAndName, -1, logger);
        Assert.Throws<SnowflakeCancellationException>(() => resultSet.Read());
        Assert.DoesNotContain(LoggedEntries(logger), entry => entry.Level >= LogLevel.Error);
        Assert.Contains(LoggedEntries(logger), entry => entry.Level == LogLevel.Debug && entry.Message.Contains("was cancelled"));

        var failure = new SnowflakeDriverException(SFError.QUERY_RESULT_FETCH_FAILED, null);
        source.FailNextWith = failure;
        Assert.Throws<SnowflakeDriverException>(() => resultSet.Read());

        var error = Assert.Single(LoggedEntries(logger), entry => entry.Level == LogLevel.Error);
        Assert.Same(failure, error.Exception);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void BatchRejectedByBindIsDisposed()
    {
        var memory = new TrackedMemory();
        using var resultSet = ResultSet(new FakeRecordBatchSource(TrackedBatch(memory)), IdAndName);

        AssertDriverError(SFError.INTERNAL_ERROR, () => resultSet.Read());

        Assert.Equal(1, memory.DisposeCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ClosingDisposesTheBatchesHasRowsPeekedAndTheCurrentOne()
    {
        var peeked = new TrackedMemory();
        var descriptor = WithQueryId(UnitBatches.Descriptor(UnitBatches.FixedColumn("ID")));
        var resultSet = ResultSet(new FakeRecordBatchSource(TrackedBatch(peeked)), descriptor);
        Assert.True(resultSet.GetHasRows());

        resultSet.Close();

        Assert.Equal(1, peeked.DisposeCalls);

        var current = new TrackedMemory();
        resultSet = ResultSet(new FakeRecordBatchSource(TrackedBatch(current)), descriptor);
        Assert.True(resultSet.Read());

        resultSet.Close();

        Assert.Equal(1, current.DisposeCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task BatchFetchedWhileTheResultSetClosesIsDisposedAndTheReadFailsAsync()
    {
        var memory = new TrackedMemory();
        var fetch = new TaskCompletionSource<RecordBatch?>();
        var source = Substitute.For<IRecordBatchSource>();
        source.NextAsync(Arg.Any<CancellationToken>()).Returns(fetch.Task);
        var descriptor = WithQueryId(UnitBatches.Descriptor(UnitBatches.FixedColumn("ID")));
        using var resultSet = ResultSet(source, descriptor);
        var read = resultSet.ReadNextBatchAsync(CancellationToken.None);

        resultSet.Close();
        fetch.SetResult(TrackedBatch(memory));

        var failure = await Assert.ThrowsAsync<SnowflakeDriverException>(() => read).ConfigureAwait(false);
        Assert.Equal(SFError.DATA_READER_ALREADY_CLOSED, failure.Error);
        Assert.Equal(1, memory.DisposeCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = ConverterSkipReason)]
    public void ValuesAndNullsComeFromTheCurrentRow()
    {
        using var resultSet = ResultSet(new FakeRecordBatchSource(IdNameBatch((1, null))), IdAndName);
        resultSet.Read();

        Assert.Equal(1L, resultSet.GetValue(0));
        Assert.Same(DBNull.Value, resultSet.GetValue(1));
        Assert.False(resultSet.IsDBNull(0));
        Assert.True(resultSet.IsDBNull(1));
        Assert.Null(resultSet.GetFieldValue<string>(1));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = ConverterSkipReason)]
    public void AColumnOfAnUnsupportedTypeDoesNotBlockTheOthers()
    {
        var descriptor = UnitBatches.Descriptor(UnitBatches.FixedColumn("ID"), UnitBatches.Column("D", "date"));
        var batch = UnitBatches.Batch(UnitBatches.Int64(5), new Date32Array.Builder().AppendNull().Build());
        using var resultSet = ResultSet(new FakeRecordBatchSource(batch), descriptor);
        resultSet.Read();

        Assert.Equal(5L, resultSet.GetFieldValue<long>(0));
        Assert.True(resultSet.IsDBNull(1));
        Assert.Throws<NotImplementedException>(() => resultSet.GetValue(1));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ScalarPropertiesReportTheResult()
    {
        using var resultSet = ResultSet(new FakeRecordBatchSource(), IdAndName, recordsAffected: 12);

        Assert.Equal(12, resultSet.RecordsAffected);
        Assert.Equal(QueryId, resultSet.QueryId);
        Assert.False(resultSet.IsClosed);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ABatchWithADifferentColumnCountIsAnInternalErrorAndIsReleased()
    {
        using var resultSet = ResultSet(new FakeRecordBatchSource(UnitBatches.Batch(UnitBatches.Int64(1))), IdAndName);

        AssertDriverError(SFError.INTERNAL_ERROR, () => resultSet.Read());
        AssertDriverError(SFError.DATA_READER_NO_CURRENT_ROW, () => resultSet.GetValue(0));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = ConverterSkipReason)]
    public void EachValueTypeIsReadByTheMatchingConverterGetter()
    {
        var guid = Guid.Parse("4b7e2c0e-8f3a-4d55-9b1e-2a6c7d8e9f01");
        var descriptor = WithQueryId(UnitBatches.Descriptor(UnitBatches.TextColumn("NUMBER"), UnitBatches.TextColumn("ID")));
        var batch = UnitBatches.Batch(UnitBatches.Strings("7"), UnitBatches.Strings(guid.ToString()));
        using var resultSet = ResultSet(new FakeRecordBatchSource(batch), descriptor);
        resultSet.Read();

        Assert.Equal((byte)7, resultSet.GetFieldValue<byte>(0));
        Assert.Equal('7', resultSet.GetFieldValue<char>(0));
        Assert.Equal(7m, resultSet.GetFieldValue<decimal>(0));
        Assert.Equal(7d, resultSet.GetFieldValue<double>(0));
        Assert.Equal(7f, resultSet.GetFieldValue<float>(0));
        Assert.Equal((short)7, resultSet.GetFieldValue<short>(0));
        Assert.Equal(7, resultSet.GetFieldValue<int>(0));
        Assert.Equal(7L, resultSet.GetFieldValue<long>(0));
        Assert.Equal("7", resultSet.GetFieldValue<string>(0));
        Assert.Equal(guid, resultSet.GetFieldValue<Guid>(1));
        Assert.Equal(nameof(Boolean), AssertDriverError(SFError.INVALID_DATA_CONVERSION, () => resultSet.GetFieldValue<bool>(0)).Args[2]);
        Assert.Equal(nameof(DateTime), AssertDriverError(SFError.INVALID_DATA_CONVERSION, () => resultSet.GetFieldValue<DateTime>(0)).Args[2]);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = ConverterSkipReason)]
    public void ATypeTheColumnCannotBeReadAsIsAConversionError()
    {
        using var resultSet = ResultSet(new FakeRecordBatchSource(IdNameBatch((1, "a"))), IdAndName);
        resultSet.Read();

        Assert.Equal(nameof(TimeSpan), AssertDriverError(SFError.INVALID_DATA_CONVERSION, () => resultSet.GetFieldValue<TimeSpan>(0)).Args[2]);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = ConverterSkipReason)]
    public void ReadAsObjectAValueIsItsNaturalTypeAndNullIsDbNull()
    {
        using var resultSet = ResultSet(new FakeRecordBatchSource(IdNameBatch((1, null))), IdAndName);
        resultSet.Read();

        Assert.Equal(1L, Assert.IsType<long>(resultSet.GetFieldValue<object>(0)));
        Assert.Same(DBNull.Value, resultSet.GetFieldValue<object>(1));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = ConverterSkipReason)]
    public void ABaseTypeOfATypeTheConverterReadsIsServedByThatRead()
    {
        using var resultSet = ResultSet(new FakeRecordBatchSource(IdNameBatch((1, "a"))), IdAndName);
        resultSet.Read();

        Assert.Equal("a", resultSet.GetFieldValue<IComparable>(1));
    }

    private static SnowflakeDriverException AssertDriverError(SFError expected, Func<object?> operation)
    {
        var exception = Assert.Throws<SnowflakeDriverException>(operation);
        Assert.Equal(expected, exception.Error);
        Assert.Equal(QueryId, exception.QueryId);
        return exception;
    }

    private static ResultSetDescriptor WithQueryId(ResultSetDescriptor descriptor)
    {
        descriptor.QueryId = QueryId;
        return descriptor;
    }

    private static SnowflakeResultSet ResultSet(IRecordBatchSource source, ResultSetDescriptor descriptor, int recordsAffected = -1) =>
        new(ResultSetInterpreter.Instance, source, ColumnConverterFactory.Instance, descriptor, recordsAffected, NullLogger<SnowflakeResultSet>.Instance);

    private static RecordBatch IdNameBatch(params (long Id, string? Name)[] rows) =>
        UnitBatches.Batch(
            UnitBatches.Int64(rows.Select(row => (long?)row.Id).ToArray()),
            UnitBatches.Strings(rows.Select(row => row.Name).ToArray()));

    private static RecordBatch TrackedBatch(TrackedMemory memory) =>
        UnitBatches.Batch(TrackedArray.Create(UnitBatches.Int64(1), memory.Disposed));

    private static List<(LogLevel Level, string Message, Exception? Exception)> LoggedEntries(ILogger logger) =>
        logger.ReceivedCalls()
            .Where(call => call.GetMethodInfo().Name == nameof(ILogger.Log))
            .Select(call => call.GetArguments())
            .Select(arguments => ((LogLevel)arguments[0]!, arguments[2]!.ToString()!, (Exception?)arguments[3]))
            .ToList();

    private sealed class TrackedMemory
    {
        private int _disposeCalls;

        public int DisposeCalls => Volatile.Read(ref _disposeCalls);

        public void Disposed() => Interlocked.Increment(ref _disposeCalls);
    }
}
