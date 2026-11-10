using System.Data.Common;
using Google.Protobuf;
using Microsoft.Extensions.Logging;
using NSubstitute;
using Snowflake.Data.Core;
using Snowflake.Data.Core.Errors;
using Snowflake.Data.Core.Handles;
using Snowflake.Data.Core.Session;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Tests.Core;

[Trait("Category", "Unit")]
public sealed class SnowflakeStatementTest
{
    private const string ExecutedQueryId = "01bf4a3e-0001-2c4f-0000-00a1b2c3d4e5";
    private const string FailedQueryId = "01bf4a3e-0002-2c4f-0000-00a1b2c3d4e6";
    private const string Sql = "INSERT INTO t VALUES (1)";

    private readonly IConnectionHandleWrapper _connection;
    private readonly IDatabaseDriverService _driver;
    private readonly IQueryStatusPoller _poller;
    private readonly ILogger<SnowflakeStatement> _logger;
    private readonly SnowflakeStatement _statement;
    private readonly IResultSetInterpreter _statementInterpreter;
    private readonly IResultSetReaderFactory _readerFactory;
    private readonly DbDataReader _reader;

    private static readonly ConnectionHandle ConnHandle = new() { Id = 2, Magic = 202 };
    private static readonly StatementHandle StmtHandle = new() { Id = 3, Magic = 303 };
    private static readonly ResultSetHandle ResultHandle = new() { Id = 4, Magic = 404 };
    private static readonly ArrowArrayStreamPtr Stream = new() { Value = ByteString.CopyFrom(BitConverter.GetBytes(0x5EED_0000L)) };
    private static readonly QueryId AsyncQueryId = QueryId.Parse("01bf4a3e-0003-2c4f-0000-00a1b2c3d4e7");
    private static readonly SnowflakeStatementCtx PlainCtx = new(Sql, null, null);

    public SnowflakeStatementTest()
    {
        _connection = Substitute.For<IConnectionHandleWrapper>();
        _connection.ConnHandle.Returns(ConnHandle);
        _statementInterpreter = Substitute.For<IResultSetInterpreter>();
        _driver = Substitute.For<IDatabaseDriverService>();
        _driver.StatementNewAsync(Arg.Any<StatementNewRequest>(), Arg.Any<CancellationToken>())
            .Returns(new StatementNewResponse { StmtHandle = StmtHandle });
        _poller = Substitute.For<IQueryStatusPoller>();
        _logger = Substitute.For<ILogger<SnowflakeStatement>>();
        _reader = Substitute.For<DbDataReader>();
        _readerFactory = Substitute.For<IResultSetReaderFactory>();
        _readerFactory.Create(default!, default!, default!, default).ReturnsForAnyArgs(_reader);
        _readerFactory.CreateForSchemaOnly(default!, default!, default).ReturnsForAnyArgs(_reader);

        _statement = new SnowflakeStatement(_connection, _statementInterpreter, _poller, _driver, _readerFactory, _logger);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ExecuteNonQueryReturnsRowsAffectedAndReleasesBothHandles()
    {
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>()).Returns(SingleResult(ExecutedQueryId, rowsAffected: 5));
        _statementInterpreter.CalculateUpdateCount(Arg.Is<ResultSetDescriptor>(descriptor => descriptor.RowsAffected == 5)).Returns(5);

        var count = _statement.ExecuteNonQuery(PlainCtx, null);

        Assert.Equal(5, count);
        Assert.Equal(ExecutedQueryId, _statement.LastQueryId);
        Received.InOrder(() =>
        {
            _driver.StatementNewAsync(Arg.Is<StatementNewRequest>(request => request.ConnHandle.Equals(ConnHandle)), Arg.Any<CancellationToken>());
            _driver.StatementSetSqlQueryAsync(Arg.Is<StatementSetSqlQueryRequest>(request => request.StmtHandle.Equals(StmtHandle) && request.Query == Sql), Arg.Any<CancellationToken>());
            _driver.StatementExecuteQueryAsync(Arg.Is<StatementExecuteQueryRequest>(request => request.StmtHandle.Equals(StmtHandle)), Arg.Any<CancellationToken>());
            _driver.ResultSetRelease(Arg.Is<ResultSetReleaseRequest>(request => request.ResultSetHandle.Equals(ResultHandle)));
            _driver.StatementRelease(Arg.Is<StatementReleaseRequest>(request => request.StmtHandle.Equals(StmtHandle)));
        });
        _ = _driver.DidNotReceiveWithAnyArgs().StatementSetOptionsAsync(default!, default);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ExecuteNonQuerySendsTheQueryTagAsTheOnlyStatementOption()
    {
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>()).Returns(SingleResult(ExecutedQueryId));

        _statement.ExecuteNonQuery(PlainCtx with { QueryTag = "nightly-load" }, null);

        _ = _driver.Received(1).StatementSetOptionsAsync(
            Arg.Is<StatementSetOptionsRequest>(request =>
                request.StmtHandle.Equals(StmtHandle)
                && request.Options.Count == 1
                && request.Options["query_tag"].StringValue == "nightly-load"),
            Arg.Any<CancellationToken>());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task ExecuteInAsyncModeSendsAsyncExecutionWithTheQueryTagAndReturnsTheQueryIdAsync()
    {
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>()).Returns(SingleResult(ExecutedQueryId));

        var queryId = await _statement.ExecuteInAsyncModeAsync(PlainCtx with { QueryTag = "nightly-load" }, CancellationToken.None).ConfigureAwait(false);

        Assert.Equal(ExecutedQueryId, queryId);
        Assert.Equal(ExecutedQueryId, _statement.LastQueryId);
        _ = _driver.Received(1).StatementSetOptionsAsync(
            Arg.Is<StatementSetOptionsRequest>(request =>
                request.Options.Count == 2
                && request.Options["async_execution"].BoolValue
                && request.Options["query_tag"].StringValue == "nightly-load"),
            Arg.Any<CancellationToken>());
        _driver.Received(1).ResultSetRelease(Arg.Is<ResultSetReleaseRequest>(request => request.ResultSetHandle.Equals(ResultHandle)));
        _driver.Received(1).StatementRelease(Arg.Is<StatementReleaseRequest>(request => request.StmtHandle.Equals(StmtHandle)));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task ServerErrorDuringExecutionIsMappedAndReleasesTheStatementAsync()
    {
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>()).Returns(Task.FromException<ExecuteQueryResponse>(ServerError("SQL compilation error", 1003, FailedQueryId)));

        var exception = await Assert.ThrowsAsync<SnowflakeServerException>(
            () => _statement.ExecuteNonQueryAsync(PlainCtx, null, CancellationToken.None)).ConfigureAwait(false);

        Assert.Equal("SQL compilation error", exception.Message);
        Assert.Equal(FailedQueryId, exception.QueryId);
        Assert.Equal(FailedQueryId, _statement.LastQueryId);
        _driver.DidNotReceiveWithAnyArgs().ResultSetRelease(default!);
        _driver.ReceivedWithAnyArgs(1).StatementRelease(default!);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void FailureWhileConfiguringTheStatementReleasesItWithoutExecuting()
    {
        _driver.StatementSetSqlQueryAsync(Arg.Any<StatementSetSqlQueryRequest>(), Arg.Any<CancellationToken>())
            .Returns(Task.FromException<StatementSetSqlQueryResponse>(new ServiceException(new DriverException { Message = "rejected" })));

        var exception = Assert.Throws<SnowflakeDriverException>(() => _statement.ExecuteNonQuery(PlainCtx, null));

        Assert.Equal(SFError.INTERNAL_ERROR, exception.Error);
        _ = _driver.DidNotReceiveWithAnyArgs().StatementExecuteQueryAsync(default!, default);
        _driver.Received(1).StatementRelease(Arg.Is<StatementReleaseRequest>(request => request.StmtHandle.Equals(StmtHandle)));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ExecutionOnAConnectionThatIsNotOpenFailsWithoutCallingCore()
    {
        _connection.ConnHandle.Returns((ConnectionHandle?)null);

        var exception = Assert.Throws<SnowflakeDriverException>(() => _statement.ExecuteNonQuery(PlainCtx, null));

        Assert.Equal(SFError.EXECUTE_COMMAND_ON_CLOSED_CONNECTION, exception.Error);
        Assert.Empty(_driver.ReceivedCalls());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CarrierRaisedInsideTheStatementKeepsTheStackTraceOfItsThrowSite()
    {
        _connection.ConnHandle.Returns((ConnectionHandle?)null);

        var exception = Assert.Throws<SnowflakeDriverException>(() => _statement.ExecuteNonQuery(PlainCtx, null));

        Assert.Contains("GetConnectionHandle", exception.StackTrace);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void UnsupportedResultKeepsTheStackTraceOfItsThrowSite()
    {
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>())
            .Returns(new ExecuteQueryResponse { Multi = new MultiStatementResult() });

        var exception = Assert.Throws<NotImplementedException>(() => _statement.ExecuteNonQuery(PlainCtx, null));

        Assert.Contains("GetSingleResult", exception.StackTrace);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void UnexpectedExceptionFromACollaboratorReachesTheCallerAsTheSameInstance()
    {
        var bug = new InvalidOperationException("interpreter bug");
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>())
            .Returns(SingleResult(ExecutedQueryId));
        _statementInterpreter.CalculateUpdateCount(Arg.Any<ResultSetDescriptor>()).Returns(_ => throw bug);

        var exception = Assert.Throws<InvalidOperationException>(() => _statement.ExecuteNonQuery(PlainCtx, null));

        Assert.Same(bug, exception);
        _driver.ReceivedWithAnyArgs(1).StatementRelease(default!);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void StatusLookupOnAClosedConnectionCarriesTheRequestedQueryId()
    {
        _connection.ConnHandle.Returns((ConnectionHandle?)null);

        var exception = Assert.Throws<SnowflakeDriverException>(() => _statement.GetQueryStatus(AsyncQueryId));

        Assert.Equal(SFError.EXECUTE_COMMAND_ON_CLOSED_CONNECTION, exception.Error);
        Assert.Equal(AsyncQueryId.Value, exception.QueryId);
        Assert.Contains("GetConnectionHandle", exception.StackTrace);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TransportFailureDuringAStatusLookupCarriesTheRequestedQueryId()
    {
        _poller.FetchStatusAsync(default!, default, default).ReturnsForAnyArgs(Task.FromException<QueryStatus>(new TransportException("transport down")));

        var exception = Assert.Throws<SnowflakeDriverException>(() => _statement.GetQueryStatus(AsyncQueryId));

        Assert.Equal(SFError.INTERNAL_ERROR, exception.Error);
        Assert.Equal(AsyncQueryId.Value, exception.QueryId);
        Assert.IsType<TransportException>(exception.InnerException);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CarrierThatAlreadyNamesAQueryKeepsItsQueryId()
    {
        var unknownStatus = new SnowflakeDriverException(SFError.UNKNOWN_QUERY_STATUS, FailedQueryId);
        _poller.FetchStatusAsync(default!, default, default).ReturnsForAnyArgs(Task.FromException<QueryStatus>(unknownStatus));

        var exception = Assert.Throws<SnowflakeDriverException>(() => _statement.GetQueryStatus(AsyncQueryId));

        Assert.Same(unknownStatus, exception);
        Assert.Equal(FailedQueryId, exception.QueryId);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CarrierWithoutAQueryIdFromTheWaitIsEnrichedWithTheRequestedOne()
    {
        var closed = new SnowflakeDriverException(SFError.EXECUTE_COMMAND_ON_CLOSED_CONNECTION, null);
        _poller.WaitUntilFinishedAsync(default!, default, default).ReturnsForAnyArgs(Task.FromException(closed));

        var exception = Assert.Throws<SnowflakeDriverException>(() => _statement.GetResultsFromQueryId(AsyncQueryId, null));

        Assert.Same(closed, exception);
        Assert.Equal(AsyncQueryId.Value, exception.QueryId);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void MultiStatementResultIsNotSupportedAndReleasesTheStatement()
    {
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>()).Returns(new ExecuteQueryResponse { Multi = new MultiStatementResult() });

        var exception = Assert.Throws<NotImplementedException>(() => _statement.ExecuteNonQuery(PlainCtx, null));

        Assert.Contains("SNOW-4429197", exception.Message);
        _driver.ReceivedWithAnyArgs(1).StatementRelease(default!);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ReaderThatCannotOpenItsStreamReleasesTheResultSet()
    {
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>()).Returns(SingleResult(ExecutedQueryId));
        _driver.ResultSetGetStreamAsync(Arg.Any<ResultSetGetStreamRequest>(), Arg.Any<CancellationToken>())
            .Returns(Task.FromException<ResultSetGetStreamResponse>(new ServiceException(new DriverException { Message = "stream unavailable" })));

        var exception = Assert.Throws<SnowflakeDriverException>(() => _statement.ExecuteReader(PlainCtx));

        Assert.Equal(SFError.INTERNAL_ERROR, exception.Error);
        Assert.Equal(ExecutedQueryId, _statement.LastQueryId);
        Received.InOrder(() =>
        {
            _driver.ResultSetGetStreamAsync(Arg.Is<ResultSetGetStreamRequest>(request => request.ResultSetHandle.Equals(ResultHandle)), Arg.Any<CancellationToken>());
            _driver.ResultSetRelease(Arg.Is<ResultSetReleaseRequest>(request => request.ResultSetHandle.Equals(ResultHandle)));
            _driver.StatementRelease(Arg.Is<StatementReleaseRequest>(request => request.StmtHandle.Equals(StmtHandle)));
        });
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ExecuteReaderHandsTheResultSetToTheReaderWithoutReleasingIt()
    {
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>()).Returns(SingleResult(ExecutedQueryId, rowsAffected: 3));
        _driver.ResultSetGetStreamAsync(Arg.Any<ResultSetGetStreamRequest>(), Arg.Any<CancellationToken>())
            .Returns(new ResultSetGetStreamResponse { Stream = Stream });
        _statementInterpreter.CalculateRecordsAffected(Arg.Is<ResultSetDescriptor>(descriptor => descriptor.RowsAffected == 3)).Returns(3);
        NativeResultSetHandle? handedOver = null;
        _readerFactory.Create(Arg.Do<NativeResultSetHandle>(handle => handedOver = handle), Arg.Any<ArrowArrayStreamPtr>(), Arg.Any<ResultSetDescriptor>(), Arg.Any<int>());

        var reader = _statement.ExecuteReader(PlainCtx);

        Assert.Same(_reader, reader);
        Assert.Equal(ExecutedQueryId, _statement.LastQueryId);
        _readerFactory.Received(1).Create(
            Arg.Is<NativeResultSetHandle>(handle => handle.Handle.Equals(ResultHandle)),
            Stream,
            Arg.Is<ResultSetDescriptor>(descriptor => descriptor.QueryId == ExecutedQueryId),
            3);
        _readerFactory.DidNotReceiveWithAnyArgs().CreateForSchemaOnly(default!, default!, default);
        _driver.DidNotReceiveWithAnyArgs().ResultSetRelease(default!);
        _driver.Received(1).StatementRelease(Arg.Is<StatementReleaseRequest>(request => request.StmtHandle.Equals(StmtHandle)));

        handedOver!.Dispose();
        _driver.Received(1).ResultSetRelease(Arg.Is<ResultSetReleaseRequest>(request => request.ResultSetHandle.Equals(ResultHandle)));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ReaderCreationFailureReleasesTheResultSet()
    {
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>()).Returns(SingleResult(ExecutedQueryId));
        _driver.ResultSetGetStreamAsync(Arg.Any<ResultSetGetStreamRequest>(), Arg.Any<CancellationToken>())
            .Returns(new ResultSetGetStreamResponse { Stream = Stream });
        var failure = new InvalidOperationException("stream import failed");
        _readerFactory.Create(default!, default!, default!, default).ReturnsForAnyArgs(_ => throw failure);

        var exception = Assert.Throws<InvalidOperationException>(() => _statement.ExecuteReader(PlainCtx));

        Assert.Same(failure, exception);
        Received.InOrder(() =>
        {
            _driver.ResultSetRelease(Arg.Is<ResultSetReleaseRequest>(request => request.ResultSetHandle.Equals(ResultHandle)));
            _driver.StatementRelease(Arg.Is<StatementReleaseRequest>(request => request.StmtHandle.Equals(StmtHandle)));
        });
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DescribeOnlyPreparesInsteadOfExecuting()
    {
        _driver.StatementPrepareAsync(Arg.Any<StatementPrepareRequest>(), Arg.Any<CancellationToken>())
            .Returns(Task.FromException<StatementPrepareResponse>(new ServiceException(new DriverException { Message = "cannot describe" })));

        var exception = Assert.Throws<SnowflakeDriverException>(() => _statement.ExecuteReader(PlainCtx with { DescribeOnly = true }));

        Assert.Equal(SFError.INTERNAL_ERROR, exception.Error);
        _ = _driver.Received(1).StatementPrepareAsync(Arg.Is<StatementPrepareRequest>(request => request.StmtHandle.Equals(StmtHandle)), Arg.Any<CancellationToken>());
        _ = _driver.DidNotReceiveWithAnyArgs().StatementExecuteQueryAsync(default!, default);
        _driver.ReceivedWithAnyArgs(1).StatementRelease(default!);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DescribeOnlyReturnsASchemaOnlyReaderWithThePreparedColumns()
    {
        var prepared = new PrepareResult { QueryId = ExecutedQueryId, Stream = Stream, SqlState = "02000" };
        prepared.Columns.Add(new ColumnMetadata { Name = "C1", Type = "FIXED" });
        _driver.StatementPrepareAsync(Arg.Any<StatementPrepareRequest>(), Arg.Any<CancellationToken>())
            .Returns(new StatementPrepareResponse { Result = prepared });

        var reader = _statement.ExecuteReader(PlainCtx with { DescribeOnly = true });

        Assert.Same(_reader, reader);
        Assert.Equal(ExecutedQueryId, _statement.LastQueryId);
        _readerFactory.Received(1).CreateForSchemaOnly(
            Stream,
            Arg.Is<ResultSetDescriptor>(descriptor =>
                descriptor.QueryId == ExecutedQueryId
                && descriptor.SqlState == "02000"
                && descriptor.Columns.Count == 1
                && descriptor.Columns[0].Name == "C1"
                && descriptor.Columns[0].Type == "FIXED"),
            -1);
        _readerFactory.DidNotReceiveWithAnyArgs().Create(default!, default!, default!, default);
        _ = _driver.DidNotReceiveWithAnyArgs().StatementExecuteQueryAsync(default!, default);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task CancelStopsARunningExecutionAndReleasesTheStatementAsync()
    {
        var executing = Signal();
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>()).Returns(call => HangAsync<ExecuteQueryResponse>(executing, call.ArgAt<CancellationToken>(1)));
        var running = _statement.ExecuteNonQueryAsync(PlainCtx, null, CancellationToken.None);
        await executing.Task.ConfigureAwait(false);

        _statement.Cancel();

        var exception = await Assert.ThrowsAsync<SnowflakeDriverException>(() => running).ConfigureAwait(false);
        Assert.Equal(SFError.QUERY_CANCELLED, exception.Error);
        _driver.ReceivedWithAnyArgs(1).StatementRelease(default!);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task CallerTokenCancellationIsReportedAsQueryCancelledAsync()
    {
        var executing = Signal();
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>()).Returns(call => HangAsync<ExecuteQueryResponse>(executing, call.ArgAt<CancellationToken>(1)));
        using var caller = new CancellationTokenSource();
        var running = _statement.ExecuteNonQueryAsync(PlainCtx, null, caller.Token);
        await executing.Task.ConfigureAwait(false);

        caller.Cancel();

        var exception = await Assert.ThrowsAsync<SnowflakeCancellationException>(() => running).ConfigureAwait(false);
        Assert.Equal(nameof(SFError.QUERY_CANCELLED), exception.Message);
        _driver.ReceivedWithAnyArgs(1).StatementRelease(default!);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task AsyncCommandTimeoutIsReportedAsRequestTimeoutAsync()
    {
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>()).Returns(call => HangAsync<ExecuteQueryResponse>(Signal(), call.ArgAt<CancellationToken>(1)));

        var exception = await Assert.ThrowsAsync<SnowflakeCancellationException>(
            () => _statement.ExecuteNonQueryAsync(PlainCtx with { CommandTimeout = TimeSpan.FromSeconds(1) }, null, CancellationToken.None)).ConfigureAwait(false);

        Assert.Equal(nameof(SFError.REQUEST_TIMEOUT), exception.Message);
        Assert.Equal([1], exception.Args);
        _driver.ReceivedWithAnyArgs(1).StatementRelease(default!);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void SyncCommandTimeoutIsReportedAsQueryCancelled()
    {
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>()).Returns(call => HangAsync<ExecuteQueryResponse>(Signal(), call.ArgAt<CancellationToken>(1)));

        var exception = Assert.Throws<SnowflakeDriverException>(
            () => _statement.ExecuteNonQuery(PlainCtx with { CommandTimeout = TimeSpan.FromSeconds(1) }, null));

        Assert.Equal(SFError.QUERY_CANCELLED, exception.Error);
        _driver.ReceivedWithAnyArgs(1).StatementRelease(default!);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task SecondOperationWhileOneIsInFlightIsRejectedAsync()
    {
        var waiting = PollerWaitsUntilCancelled();
        var running = _statement.GetResultsFromQueryIdAsync(AsyncQueryId, null, CancellationToken.None);
        await waiting.Task.ConfigureAwait(false);

        var exception = Assert.Throws<SnowflakeDriverException>(() => _statement.ExecuteNonQuery(PlainCtx, null));

        Assert.Equal(SFError.STATEMENT_ALREADY_RUNNING_QUERY, exception.Error);
        _ = _driver.DidNotReceiveWithAnyArgs().StatementNewAsync(default!, default);
        _statement.Cancel();
        await Assert.ThrowsAsync<SnowflakeDriverException>(() => running).ConfigureAwait(false);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task StatusLookupDoesNotNeedTheInFlightSlotAsync()
    {
        var waiting = PollerWaitsUntilCancelled();
        _poller.FetchStatusAsync(default!, default, default).ReturnsForAnyArgs(QueryStatus.Running);
        var running = _statement.GetResultsFromQueryIdAsync(AsyncQueryId, null, CancellationToken.None);
        await waiting.Task.ConfigureAwait(false);

        var status = _statement.GetQueryStatus(AsyncQueryId);

        Assert.Equal(QueryStatus.Running, status);
        _statement.Cancel();
        await Assert.ThrowsAsync<SnowflakeDriverException>(() => running).ConfigureAwait(false);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void FailedExecutionFreesTheStatementForTheNextOne()
    {
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>()).Returns(
            Task.FromException<ExecuteQueryResponse>(ServerError("warehouse suspended", 606, FailedQueryId)),
            Task.FromResult(SingleResult(ExecutedQueryId, rowsAffected: 2)));
        _statementInterpreter.CalculateUpdateCount(Arg.Is<ResultSetDescriptor>(descriptor => descriptor.RowsAffected == 2)).Returns(2);
        Assert.Throws<SnowflakeServerException>(() => _statement.ExecuteNonQuery(PlainCtx, null));

        var count = _statement.ExecuteNonQuery(PlainCtx, null);

        Assert.Equal(2, count);
        Assert.Equal(ExecutedQueryId, _statement.LastQueryId);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task GetQueryStatusAsksThePollerWithTheConnectionHandleAsync()
    {
        _poller.FetchStatusAsync(default!, default, default).ReturnsForAnyArgs(QueryStatus.Queued);

        var status = await _statement.GetQueryStatusAsync(AsyncQueryId, CancellationToken.None).ConfigureAwait(false);

        Assert.Equal(QueryStatus.Queued, status);
        _ = _poller.Received(1).FetchStatusAsync(ConnHandle, AsyncQueryId, Arg.Any<CancellationToken>());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void FailedStatusLookupIsMappedWithoutChangingTheLastQueryId()
    {
        _poller.FetchStatusAsync(default!, default, default).ReturnsForAnyArgs(Task.FromException<QueryStatus>(
            new ServiceException(new DriverException { Message = "lookup failed", QueryId = FailedQueryId })));

        var exception = Assert.Throws<SnowflakeDriverException>(() => _statement.GetQueryStatus(AsyncQueryId));

        Assert.Equal(SFError.INTERNAL_ERROR, exception.Error);
        Assert.Equal(FailedQueryId, exception.QueryId);
        Assert.Null(_statement.LastQueryId);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task ResultFetchAfterTheWaitSendsTheQueryIdAndMapsAServerErrorAsync()
    {
        _driver.ConnectionGetQueryResultAsync(Arg.Any<ConnectionGetQueryResultRequest>(), Arg.Any<CancellationToken>())
            .Returns(Task.FromException<ExecuteQueryResponse>(ServerError("result expired", 2003, FailedQueryId)));

        var exception = await Assert.ThrowsAsync<SnowflakeServerException>(
            () => _statement.GetResultsFromQueryIdAsync(AsyncQueryId, null, CancellationToken.None)).ConfigureAwait(false);

        Assert.Equal("result expired", exception.Message);
        Assert.Equal(FailedQueryId, _statement.LastQueryId);
        _ = _poller.Received(1).WaitUntilFinishedAsync(Arg.Any<Func<ConnectionHandle>>(), AsyncQueryId, Arg.Any<CancellationToken>());
        _ = _driver.Received(1).ConnectionGetQueryResultAsync(
            Arg.Is<ConnectionGetQueryResultRequest>(request => request.ConnHandle.Equals(ConnHandle) && request.QueryId == AsyncQueryId.Value),
            Arg.Any<CancellationToken>());
        _ = _driver.DidNotReceiveWithAnyArgs().ConnectionAbortQueryAsync(default!, default);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task GetResultsFromQueryIdOpensAReaderOnceTheWaitEndsAsync()
    {
        _driver.ConnectionGetQueryResultAsync(Arg.Any<ConnectionGetQueryResultRequest>(), Arg.Any<CancellationToken>())
            .Returns(SingleResult(AsyncQueryId.Value, rowsAffected: 7));
        _driver.ResultSetGetStreamAsync(Arg.Any<ResultSetGetStreamRequest>(), Arg.Any<CancellationToken>())
            .Returns(new ResultSetGetStreamResponse { Stream = Stream });
        _statementInterpreter.CalculateRecordsAffected(Arg.Is<ResultSetDescriptor>(descriptor => descriptor.RowsAffected == 7)).Returns(7);

        var reader = await _statement.GetResultsFromQueryIdAsync(AsyncQueryId, null, CancellationToken.None).ConfigureAwait(false);

        Assert.Same(_reader, reader);
        Assert.Equal(AsyncQueryId.Value, _statement.LastQueryId);
        Received.InOrder(() =>
        {
            _poller.WaitUntilFinishedAsync(Arg.Any<Func<ConnectionHandle>>(), AsyncQueryId, Arg.Any<CancellationToken>());
            _readerFactory.Create(
                Arg.Is<NativeResultSetHandle>(handle => handle.Handle.Equals(ResultHandle)),
                Stream,
                Arg.Is<ResultSetDescriptor>(descriptor => descriptor.QueryId == AsyncQueryId.Value),
                7);
        });
        _driver.DidNotReceiveWithAnyArgs().ResultSetRelease(default!);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task CancelDuringTheWaitStopsWaitingWithoutAbortingTheQueryAsync()
    {
        var waiting = PollerWaitsUntilCancelled();
        var running = _statement.GetResultsFromQueryIdAsync(AsyncQueryId, null, CancellationToken.None);
        await waiting.Task.ConfigureAwait(false);

        _statement.Cancel();

        var exception = await Assert.ThrowsAsync<SnowflakeDriverException>(() => running).ConfigureAwait(false);
        Assert.Equal(SFError.QUERY_CANCELLED, exception.Error);
        Assert.Equal(AsyncQueryId.Value, exception.QueryId);
        _ = _driver.DidNotReceiveWithAnyArgs().ConnectionAbortQueryAsync(default!, default);
        _ = _driver.DidNotReceiveWithAnyArgs().ConnectionGetQueryResultAsync(default!, default);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task CallerTokenCancellationDuringTheWaitStopsWaitingWithoutAbortingTheQueryAsync()
    {
        var waiting = PollerWaitsUntilCancelled();
        using var caller = new CancellationTokenSource();
        var running = _statement.GetResultsFromQueryIdAsync(AsyncQueryId, null, caller.Token);
        await waiting.Task.ConfigureAwait(false);

        caller.Cancel();

        var exception = await Assert.ThrowsAsync<SnowflakeCancellationException>(() => running).ConfigureAwait(false);
        Assert.Equal(nameof(SFError.QUERY_CANCELLED), exception.Message);
        Assert.Equal(AsyncQueryId.Value, exception.QueryId);
        _ = _driver.DidNotReceiveWithAnyArgs().ConnectionAbortQueryAsync(default!, default);
        _ = _driver.DidNotReceiveWithAnyArgs().ConnectionGetQueryResultAsync(default!, default);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task DisposeCancelsTheWaitAndLeavesTheStatementUsableAsync()
    {
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>()).Returns(SingleResult(ExecutedQueryId, rowsAffected: 1));
        _statementInterpreter.CalculateUpdateCount(Arg.Is<ResultSetDescriptor>(descriptor => descriptor.RowsAffected == 1)).Returns(1);
        var waiting = PollerWaitsUntilCancelled();
        var running = _statement.GetResultsFromQueryIdAsync(AsyncQueryId, null, CancellationToken.None);
        await waiting.Task.ConfigureAwait(false);

        _statement.Dispose();

        var exception = await Assert.ThrowsAsync<SnowflakeDriverException>(() => running).ConfigureAwait(false);
        Assert.Equal(SFError.QUERY_CANCELLED, exception.Error);
        Assert.Equal(1, _statement.ExecuteNonQuery(PlainCtx, null));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void SyncCommandTimeoutStopsTheWaitWithoutAbortingTheQuery()
    {
        PollerWaitsUntilCancelled();

        var exception = Assert.Throws<SnowflakeDriverException>(() => _statement.GetResultsFromQueryId(AsyncQueryId, TimeSpan.FromSeconds(1)));

        Assert.Equal(SFError.QUERY_CANCELLED, exception.Error);
        Assert.Equal(AsyncQueryId.Value, exception.QueryId);
        _ = _driver.DidNotReceiveWithAnyArgs().ConnectionAbortQueryAsync(default!, default);
        _ = _driver.DidNotReceiveWithAnyArgs().ConnectionGetQueryResultAsync(default!, default);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task AsyncCommandTimeoutStopsTheWaitAsRequestTimeoutAsync()
    {
        PollerWaitsUntilCancelled();

        var exception = await Assert.ThrowsAsync<SnowflakeCancellationException>(
            () => _statement.GetResultsFromQueryIdAsync(AsyncQueryId, TimeSpan.FromSeconds(1), CancellationToken.None)).ConfigureAwait(false);

        Assert.Equal(SFError.REQUEST_TIMEOUT, exception.Error);
        Assert.Equal([1], exception.Args);
        Assert.Equal(AsyncQueryId.Value, exception.QueryId);
        _ = _driver.DidNotReceiveWithAnyArgs().ConnectionAbortQueryAsync(default!, default);
        _ = _driver.DidNotReceiveWithAnyArgs().ConnectionGetQueryResultAsync(default!, default);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CommandTimeoutDoesNotCutShortAWaitThatFinishesInTime()
    {
        _driver.ConnectionGetQueryResultAsync(Arg.Any<ConnectionGetQueryResultRequest>(), Arg.Any<CancellationToken>())
            .Returns(Task.FromException<ExecuteQueryResponse>(ServerError("result expired", 2003, FailedQueryId)));

        Assert.Throws<SnowflakeServerException>(() => _statement.GetResultsFromQueryId(AsyncQueryId, TimeSpan.FromMinutes(5)));

        _ = _driver.Received(1).ConnectionGetQueryResultAsync(Arg.Any<ConnectionGetQueryResultRequest>(), Arg.Any<CancellationToken>());
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData((int)CancellationAbortOutcome.Aborted)]
    [InlineData((int)CancellationAbortOutcome.NotRunning)]
    public void CoreCancellationDoesNotCarryTheCoreErrorDetails(int outcome)
    {
        var error = UniversalCoreErrorDetails.Error();
        error.Kind = ErrorKind.Cancelled;
        error.CancellationAbortOutcome = (CancellationAbortOutcome)outcome;
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>())
            .Returns(Task.FromException<ExecuteQueryResponse>(new ServiceException(error)));

        var exception = Assert.ThrowsAny<SnowflakeException>(() => _statement.ExecuteNonQuery(PlainCtx, null));

        Assert.Null(exception.InnerException);
        Assert.DoesNotContain(UniversalCoreErrorDetails.Marker, exception.ToString());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void WaitFailureIsMappedWithoutFetchingTheResult()
    {
        _poller.WaitUntilFinishedAsync(default!, default, default).ReturnsForAnyArgs(Task.FromException(
            new ServiceException(new DriverException { Message = "lookup failed", QueryId = FailedQueryId })));

        var exception = Assert.Throws<SnowflakeDriverException>(() => _statement.GetResultsFromQueryId(AsyncQueryId, null));

        Assert.Equal(SFError.INTERNAL_ERROR, exception.Error);
        Assert.Null(_statement.LastQueryId);
        _ = _driver.DidNotReceiveWithAnyArgs().ConnectionGetQueryResultAsync(default!, default);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DriverErrorFromTheWaitReachesTheCallerUnchanged()
    {
        var noData = new SnowflakeDriverException(SFError.QUERY_RESULT_NO_DATA_RETRIES_EXCEEDED, AsyncQueryId.Value);
        _poller.WaitUntilFinishedAsync(default!, default, default).ReturnsForAnyArgs(Task.FromException(noData));

        var exception = Assert.Throws<SnowflakeDriverException>(() => _statement.GetResultsFromQueryId(AsyncQueryId, null));

        Assert.Same(noData, exception);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CancelWithNothingInFlightDoesNothing()
    {

        _statement.Cancel();

        Assert.Empty(_driver.ReceivedCalls());
        Assert.Null(_statement.LastQueryId);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task ServerErrorWhileSubmittingInAsyncModeIsMappedAsync()
    {
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>())
            .Returns(Task.FromException<ExecuteQueryResponse>(ServerError("SQL compilation error", 1003, FailedQueryId)));

        var exception = await Assert.ThrowsAsync<SnowflakeServerException>(
            () => _statement.ExecuteInAsyncModeAsync(PlainCtx, CancellationToken.None)).ConfigureAwait(false);

        Assert.Equal(1003, exception.VendorCode);
        Assert.Equal(FailedQueryId, _statement.LastQueryId);
        _driver.ReceivedWithAnyArgs(1).StatementRelease(default!);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ResultFetchThatCannotOpenItsStreamRecordsTheResultQueryIdAndTheRequestedOne()
    {
        _driver.ConnectionGetQueryResultAsync(Arg.Any<ConnectionGetQueryResultRequest>(), Arg.Any<CancellationToken>())
            .Returns(SingleResult(ExecutedQueryId));
        _driver.ResultSetGetStreamAsync(Arg.Any<ResultSetGetStreamRequest>(), Arg.Any<CancellationToken>())
            .Returns(Task.FromException<ResultSetGetStreamResponse>(new ServiceException(new DriverException { Message = "stream unavailable" })));

        var exception = Assert.Throws<SnowflakeDriverException>(() => _statement.GetResultsFromQueryId(AsyncQueryId, null));

        Assert.Equal(SFError.INTERNAL_ERROR, exception.Error);
        Assert.Equal(AsyncQueryId.Value, exception.QueryId);
        Assert.Equal(ExecutedQueryId, _statement.LastQueryId);
        _driver.Received(1).ResultSetRelease(Arg.Is<ResultSetReleaseRequest>(request => request.ResultSetHandle.Equals(ResultHandle)));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ConnectionClosedBeforeTheResultFetchCarriesTheRequestedQueryId()
    {
        _connection.ConnHandle.Returns((ConnectionHandle?)null);

        var exception = Assert.Throws<SnowflakeDriverException>(() => _statement.GetResultsFromQueryId(AsyncQueryId, null));

        Assert.Equal(SFError.EXECUTE_COMMAND_ON_CLOSED_CONNECTION, exception.Error);
        Assert.Equal(AsyncQueryId.Value, exception.QueryId);
        Assert.Contains("GetConnectionHandle", exception.StackTrace);
        _ = _driver.DidNotReceiveWithAnyArgs().ConnectionGetQueryResultAsync(default!, default);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData((int)CancellationAbortOutcome.Aborted)]
    [InlineData((int)CancellationAbortOutcome.NotConfirmed)]
    public void CoreCancellationThatReachedARunningQueryIsReportedAsTheServerCancellation(int outcome)
    {
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>())
            .Returns(Task.FromException<ExecuteQueryResponse>(CoreCancellation(FailedQueryId, (CancellationAbortOutcome?)outcome)));

        var exception = Assert.Throws<SnowflakeServerException>(() => _statement.ExecuteNonQuery(PlainCtx, null));

        Assert.Equal("57014", exception.SqlState);
        Assert.Equal(604, exception.VendorCode);
        Assert.Equal("SQL execution canceled", exception.Message);
        Assert.Equal(FailedQueryId, exception.QueryId);
        Assert.Equal(FailedQueryId, _statement.LastQueryId);
        Assert.Null(exception.InnerException);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData((int)CancellationAbortOutcome.NotRunning)]
    [InlineData(null)]
    public void CoreCancellationThatDidNotReachARunningQueryIsReportedAsQueryCancelled(int? outcome)
    {
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>())
            .Returns(Task.FromException<ExecuteQueryResponse>(CoreCancellation(FailedQueryId, (CancellationAbortOutcome?)outcome)));

        var exception = Assert.Throws<SnowflakeDriverException>(() => _statement.ExecuteNonQuery(PlainCtx, null));

        Assert.Equal(SFError.QUERY_CANCELLED, exception.Error);
        Assert.Equal(FailedQueryId, exception.QueryId);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CoreCancellationWithoutAQueryIdLeavesItUnset()
    {
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>())
            .Returns(Task.FromException<ExecuteQueryResponse>(CoreCancellation(null, null)));

        var exception = Assert.Throws<SnowflakeDriverException>(() => _statement.ExecuteNonQuery(PlainCtx, null));

        Assert.Null(exception.QueryId);
        Assert.Null(_statement.LastQueryId);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task TransportFailureAfterCancelIsReportedAsQueryCancelledAsync()
    {
        var executing = Signal();
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>())
            .Returns(call => TransportFailureOnCancellationAsync<ExecuteQueryResponse>(executing, call.ArgAt<CancellationToken>(1)));
        var running = _statement.ExecuteNonQueryAsync(PlainCtx, null, CancellationToken.None);
        await executing.Task.ConfigureAwait(false);

        _statement.Cancel();

        var exception = await Assert.ThrowsAsync<SnowflakeDriverException>(() => running).ConfigureAwait(false);
        Assert.Equal(SFError.QUERY_CANCELLED, exception.Error);
        Assert.IsType<TransportException>(exception.InnerException);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TransportFailureWithoutCancellationIsAnInternalError()
    {
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>())
            .Returns(Task.FromException<ExecuteQueryResponse>(new TransportException("transport down")));

        var exception = Assert.Throws<SnowflakeDriverException>(() => _statement.ExecuteNonQuery(PlainCtx, null));

        Assert.Equal(SFError.INTERNAL_ERROR, exception.Error);
        Assert.IsType<TransportException>(exception.InnerException);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task CancelOfAnAsyncExecutionWithATimeoutIsNotReportedAsATimeoutAsync()
    {
        var executing = Signal();
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>())
            .Returns(call => HangAsync<ExecuteQueryResponse>(executing, call.ArgAt<CancellationToken>(1)));
        var running = _statement.ExecuteNonQueryAsync(PlainCtx with { CommandTimeout = TimeSpan.FromMinutes(5) }, null, CancellationToken.None);
        await executing.Task.ConfigureAwait(false);

        _statement.Cancel();

        var exception = await Assert.ThrowsAsync<SnowflakeDriverException>(() => running).ConfigureAwait(false);
        Assert.Equal(SFError.QUERY_CANCELLED, exception.Error);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task CancelEndsTheRunningExecutionPromptlyAsync()
    {
        var executing = Signal();
        _driver.StatementExecuteQueryAsync(Arg.Any<StatementExecuteQueryRequest>(), Arg.Any<CancellationToken>())
            .Returns(call => HangAsync<ExecuteQueryResponse>(executing, call.ArgAt<CancellationToken>(1)));
        var running = _statement.ExecuteNonQueryAsync(PlainCtx, null, CancellationToken.None);
        await executing.Task.ConfigureAwait(false);

        _statement.Cancel();

        var finished = await Task.WhenAny(running, Task.Delay(TimeSpan.FromSeconds(10), CancellationToken.None)).ConfigureAwait(false);
        Assert.Same(running, finished);
        Assert.True(running.IsFaulted);
    }

    private static ExecuteQueryResponse SingleResult(string queryId, long? rowsAffected = null)
    {
        var descriptor = new ResultSetDescriptor { QueryId = queryId };
        if (rowsAffected is { } rows)
            descriptor.RowsAffected = rows;

        return new ExecuteQueryResponse
        {
            Single = new ResultSetResponse { ResultSetHandle = ResultHandle, ResultDescriptor = descriptor },
        };
    }

    private static ServiceException ServerError(string message, int vendorCode, string queryId) =>
        new(new DriverException { Message = message, VendorCode = vendorCode, SqlState = "42000", QueryId = queryId });

    private static TaskCompletionSource<bool> Signal() => new(TaskCreationOptions.RunContinuationsAsynchronously);

    private static async Task<T> HangAsync<T>(TaskCompletionSource<bool> started, CancellationToken cancellationToken)
    {
        started.TrySetResult(true);
        await Task.Delay(Timeout.Infinite, cancellationToken).ConfigureAwait(false);
        return default!;
    }

    private static async Task<T> TransportFailureOnCancellationAsync<T>(TaskCompletionSource<bool> started, CancellationToken cancellationToken)
    {
        started.TrySetResult(true);
        var cancelled = new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);
        using (cancellationToken.Register(() => cancelled.TrySetResult(true)))
            await cancelled.Task.ConfigureAwait(false);

        throw new TransportException("transport closed by cancellation");
    }

    private static ServiceException CoreCancellation(string? queryId, CancellationAbortOutcome? outcome)
    {
        var error = new DriverException { Message = "cancelled", Kind = ErrorKind.Cancelled };
        if (queryId is not null)
            error.QueryId = queryId;
        if (outcome is { } abortOutcome)
            error.CancellationAbortOutcome = abortOutcome;

        return new ServiceException(error);
    }

    /// <summary>Makes every poller wait last until its token is cancelled; the result completes when a wait starts.</summary>
    private TaskCompletionSource<bool> PollerWaitsUntilCancelled()
    {
        var started = Signal();
        _poller.WaitUntilFinishedAsync(default!, default, default)
            .ReturnsForAnyArgs(call => HangAsync<bool>(started, call.ArgAt<CancellationToken>(2)));
        return started;
    }
}
