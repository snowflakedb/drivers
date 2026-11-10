using System.Data.Common;
using System.Diagnostics;
using Microsoft.Extensions.Logging;
using Snowflake.Data.Core.Errors;
using Snowflake.Data.Core.Extensions;
using Snowflake.Data.Core.Handles;
using Snowflake.Data.Core.Session;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Interop;
using Snowflake.Data.Proto;
using Snowflake.Data.Proto.Errors;
using Snowflake.Data.Telemetry;

namespace Snowflake.Data.Core;

/// <summary>
/// Runs the operations of one <see cref="SnowflakeDbCommand"/> against sf_core.
/// </summary>
/// <remarks>
/// Every execution creates its own sf_core statement handle and releases it as soon as the
/// execution returns; an open reader does not depend on it. All sf_core calls go through the
/// asynchronous FFI so that <see cref="Cancel"/> and command timeouts can interrupt them; the
/// synchronous methods block on that path through <see cref="SyncOverAsync"/>.
/// </remarks>
internal sealed class SnowflakeStatement : ISnowflakeStatement
{
    private const int QueryCanceledSqlState = 57014;
    private const int QueryCanceledVendorCode = 604;
    private const string QueryCanceledMessage = "SQL execution canceled";

    private readonly IConnectionHandleWrapper _connection;
    private readonly IResultSetInterpreter _resultSetInterpreter;
    private readonly IQueryStatusPoller _queryStatusPoller;
    private readonly IDatabaseDriverService _driverService;
    private readonly IResultSetReaderFactory _resultSetReaderFactory;
    private readonly ILogger<SnowflakeStatement> _logger;
    private readonly LockObject _lock = new();

    private OperationScope? _inFlight;
    private volatile string? _lastQueryId;

    internal SnowflakeStatement(
        IConnectionHandleWrapper connection,
        IResultSetInterpreter resultSetInterpreter,
        IQueryStatusPoller queryStatusPoller,
        IDatabaseDriverService driverService,
        IResultSetReaderFactory resultSetReaderFactory,
        ILogger<SnowflakeStatement> logger)
    {
        _connection = connection;
        _resultSetInterpreter = resultSetInterpreter;
        _queryStatusPoller = queryStatusPoller;
        _driverService = driverService;
        _resultSetReaderFactory = resultSetReaderFactory;
        _logger = logger;
    }

    internal SnowflakeStatement(SnowflakeDbConnection connection)
        : this(connection, ResultSetInterpreter.Instance,
            new QueryStatusPoller(new DatabaseDriverServiceClient(SfCoreTransport.Instance), QueryResultsRetryConfig.GetDefault),
            new DatabaseDriverServiceClient(SfCoreTransport.Instance),
            ResultSetReaderFactory.Instance,
            LoggerAndTelemetryStubs.GetLogger<SnowflakeStatement>())
    {
    }

    public string? LastQueryId => _lastQueryId;

    public void Cancel()
    {
        OperationScope? inFlight;
        lock (_lock)
            inFlight = _inFlight;

        // Outside the lock: cancellation runs the continuations of the cancelled operation inline.
        inFlight?.RequestCancel();
    }

    /// <summary>Cancels the operation in flight; the statement stays usable - backward compatibility.</summary>
    public void Dispose() => Cancel();

    public int ExecuteNonQuery(SnowflakeStatementCtx ctx, Activity? activity)
    {
        using var scope = Register(OperationScope.Sync(ctx.CommandTimeout));
        return SyncOverAsync.Wait(ExecuteNonQueryCoreAsync(ctx, activity, scope, scope.Token));
    }

    public async Task<int> ExecuteNonQueryAsync(SnowflakeStatementCtx ctx, Activity? activity, CancellationToken cancellationToken)
    {
        using var scope = Register(OperationScope.Async(ctx.CommandTimeout, cancellationToken));
        return await ExecuteNonQueryCoreAsync(ctx, activity, scope, scope.Token).ConfigureAwait(false);
    }

    public DbDataReader ExecuteReader(SnowflakeStatementCtx ctx)
    {
        using var scope = Register(OperationScope.Sync(ctx.CommandTimeout));
        return SyncOverAsync.Wait(ExecuteReaderCoreAsync(ctx, scope, scope.Token));
    }

    public async Task<DbDataReader> ExecuteReaderAsync(SnowflakeStatementCtx ctx, CancellationToken cancellationToken)
    {
        using var scope = Register(OperationScope.Async(ctx.CommandTimeout, cancellationToken));
        return await ExecuteReaderCoreAsync(ctx, scope, scope.Token).ConfigureAwait(false);
    }

    public string ExecuteInAsyncMode(SnowflakeStatementCtx ctx)
    {
        using var scope = Register(OperationScope.Sync(ctx.CommandTimeout));
        return SyncOverAsync.Wait(ExecuteInAsyncModeCoreAsync(ctx, scope, scope.Token));
    }

    public async Task<string> ExecuteInAsyncModeAsync(SnowflakeStatementCtx ctx, CancellationToken cancellationToken)
    {
        using var scope = Register(OperationScope.Async(ctx.CommandTimeout, cancellationToken));
        return await ExecuteInAsyncModeCoreAsync(ctx, scope, scope.Token).ConfigureAwait(false);
    }

    public QueryStatus GetQueryStatus(QueryId queryId)
    {
        // A status lookup does not register: it does not occupy the in-flight slot and Cancel() does not reach it.
        using var scope = OperationScope.Sync(null);
        return SyncOverAsync.Wait(GetQueryStatusCoreAsync(queryId, scope, scope.Token));
    }

    public async Task<QueryStatus> GetQueryStatusAsync(QueryId queryId, CancellationToken cancellationToken)
    {
        // A status lookup does not register: it does not occupy the in-flight slot and Cancel() does not reach it.
        using var scope = OperationScope.Async(null, cancellationToken);
        return await GetQueryStatusCoreAsync(queryId, scope, scope.Token).ConfigureAwait(false);
    }

    public DbDataReader GetResultsFromQueryId(QueryId queryId, TimeSpan? timeout)
    {
        using var scope = Register(OperationScope.Sync(timeout));
        return SyncOverAsync.Wait(GetResultsFromQueryIdCoreAsync(queryId, scope, scope.Token));
    }

    public async Task<DbDataReader> GetResultsFromQueryIdAsync(QueryId queryId, TimeSpan? timeout, CancellationToken cancellationToken)
    {
        using var scope = Register(OperationScope.Async(timeout, cancellationToken));
        return await GetResultsFromQueryIdCoreAsync(queryId, scope, scope.Token).ConfigureAwait(false);
    }

    private async Task<int> ExecuteNonQueryCoreAsync(SnowflakeStatementCtx ctx, Activity? activity, OperationScope scope, CancellationToken cancellationToken)
    {
        try
        {
            using var statement = await CreateNativeStatementAsync(ctx, asyncExecution: false, cancellationToken).ConfigureAwait(false);
            var request = statement.BuildExecuteQueryRequest();
            var response = await _driverService.StatementExecuteQueryAsync(request, cancellationToken).ConfigureAwait(false);
            var resultSet = GetSingleResult(response);
            using var resultSetHandle = new NativeResultSetHandle(_driverService, resultSet.ResultSetHandle, _logger);

            var descriptor = resultSet.ResultDescriptor;
            SetLastQueryId(descriptor.QueryId);
            var count = _resultSetInterpreter.CalculateUpdateCount(descriptor);
            if (count < 0)
            {
                // exceeded max int, return -1
                activity?.AddTelemetryEvent(TelemetryEvents.RowCountNegative);
            }

            return count;
        }
        catch (Exception ex) when (UniversalCoreExceptions.IsUniversalCoreException(ex))
        {
            throw MapUniversalCoreException(ex, scope);
        }
    }

    private async Task<DbDataReader> ExecuteReaderCoreAsync(SnowflakeStatementCtx ctx, OperationScope scope, CancellationToken cancellationToken)
    {
        try
        {
            using var statement = await CreateNativeStatementAsync(ctx, asyncExecution: false, cancellationToken).ConfigureAwait(false);

            if (ctx.DescribeOnly)
                return await DescribeAsync(statement, cancellationToken).ConfigureAwait(false);

            var request = statement.BuildExecuteQueryRequest();
            var response = await _driverService.StatementExecuteQueryAsync(request, cancellationToken).ConfigureAwait(false);
            var resultSet = GetSingleResult(response);
            SetLastQueryId(resultSet.ResultDescriptor.QueryId);
            return await OpenReaderAsync(resultSet, cancellationToken).ConfigureAwait(false);
        }
        catch (Exception ex) when (UniversalCoreExceptions.IsUniversalCoreException(ex))
        {
            throw MapUniversalCoreException(ex, scope);
        }
    }

    private async Task<string> ExecuteInAsyncModeCoreAsync(SnowflakeStatementCtx ctx, OperationScope scope, CancellationToken cancellationToken)
    {
        try
        {
            using var statement = await CreateNativeStatementAsync(ctx, asyncExecution: true, cancellationToken).ConfigureAwait(false);
            var request = statement.BuildExecuteQueryRequest();
            var response = await _driverService.StatementExecuteQueryAsync(request, cancellationToken).ConfigureAwait(false);
            var resultSet = GetSingleResult(response);
            using var resultSetHandle = new NativeResultSetHandle(_driverService, resultSet.ResultSetHandle, _logger);
            SetLastQueryId(resultSet.ResultDescriptor.QueryId);
            return resultSet.ResultDescriptor.QueryId;
        }
        catch (Exception ex) when (UniversalCoreExceptions.IsUniversalCoreException(ex))
        {
            throw MapUniversalCoreException(ex, scope);
        }
    }

    private async Task<QueryStatus> GetQueryStatusCoreAsync(QueryId queryId, OperationScope scope, CancellationToken cancellationToken)
    {
        try
        {
            return await _queryStatusPoller.FetchStatusAsync(GetConnectionHandle(), queryId, cancellationToken).ConfigureAwait(false);
        }
        catch (Exception ex) when (UniversalCoreExceptions.IsUniversalCoreException(ex))
        {
            var toThrow = MapUniversalCoreException(ex, scope, mapLastQuery: false);
            EnrichException(toThrow, queryId);
            throw toThrow;
        }
        catch (SnowflakeException ex)
        {
            EnrichException(ex, queryId);
            throw;
        }
    }

    private async Task<DbDataReader> GetResultsFromQueryIdCoreAsync(QueryId queryId, OperationScope scope, CancellationToken cancellationToken)
    {
        try
        {
            await _queryStatusPoller.WaitUntilFinishedAsync(GetConnectionHandle, queryId, cancellationToken).ConfigureAwait(false);
        }
        catch (Exception ex) when (UniversalCoreExceptions.IsUniversalCoreException(ex))
        {
            var toThrow = MapUniversalCoreException(ex, scope, mapLastQuery: false);
            EnrichException(toThrow, queryId);
            throw toThrow;
        }
        catch (SnowflakeException ex)
        {
            EnrichException(ex, queryId);
            throw;
        }

        try
        {
            var request = new ConnectionGetQueryResultRequest { ConnHandle = GetConnectionHandle(), QueryId = queryId.Value };
            var response = await _driverService.ConnectionGetQueryResultAsync(request, cancellationToken).ConfigureAwait(false);
            var resultSet = GetSingleResult(response);
            SetLastQueryId(resultSet.ResultDescriptor.QueryId);
            return await OpenReaderAsync(resultSet, cancellationToken).ConfigureAwait(false);
        }
        catch (Exception ex) when (UniversalCoreExceptions.IsUniversalCoreException(ex))
        {
            var toThrow = MapUniversalCoreException(ex, scope);
            EnrichException(toThrow, queryId);
            throw toThrow;
        }
        catch (SnowflakeException ex)
        {
            EnrichException(ex, queryId);
            throw;
        }
    }

    private async Task<DbDataReader> DescribeAsync(NativeStatementHandle statement, CancellationToken cancellationToken)
    {
        var request = statement.BuildPrepareRequest();
        var response = await _driverService.StatementPrepareAsync(request, cancellationToken).ConfigureAwait(false);
        var prepared = response.Result;
        SetLastQueryId(prepared.QueryId);

        var descriptor = new ResultSetDescriptor { QueryId = prepared.QueryId };
        descriptor.Columns.Add(prepared.Columns);

        if (prepared.HasSqlState)
            descriptor.SqlState = prepared.SqlState;

        return _resultSetReaderFactory.CreateForSchemaOnly(prepared.Stream, descriptor, -1);
    }

    private async Task<DbDataReader> OpenReaderAsync(ResultSetResponse resultSet, CancellationToken cancellationToken)
    {
        var resultSetHandle = new NativeResultSetHandle(_driverService, resultSet.ResultSetHandle, _logger);
        try
        {
            var streamResponse = await _driverService.ResultSetGetStreamAsync(
                resultSetHandle.BuildGetStreamRequest(), cancellationToken).ConfigureAwait(false);
            var descriptor = resultSet.ResultDescriptor;
            var calculateRecordsAffected = _resultSetInterpreter.CalculateRecordsAffected(descriptor);
            return _resultSetReaderFactory.Create(resultSetHandle, streamResponse.Stream, descriptor, calculateRecordsAffected);
        }
        catch
        {
            resultSetHandle.Dispose();
            throw;
        }
    }

    private async Task<NativeStatementHandle> CreateNativeStatementAsync(SnowflakeStatementCtx ctx, bool asyncExecution, CancellationToken cancellationToken)
    {
        var statementRequest = new StatementNewRequest { ConnHandle = GetConnectionHandle() };
        var statementResponse = await _driverService.StatementNewAsync(statementRequest, cancellationToken).ConfigureAwait(false);
        var statement = new NativeStatementHandle(_driverService, statementResponse.StmtHandle, _logger);
        try
        {
            await _driverService.StatementSetSqlQueryAsync(statement.BuildSetSqlQueryRequest(ctx.CommandText), cancellationToken).ConfigureAwait(false);

            var options = statement
                .BuildSetOptionsRequest()
                .WithQueryTag(ctx.QueryTag)
                .WithAsyncExecution(asyncExecution);

            if (!options.IsEmpty)
                await _driverService.StatementSetOptionsAsync(options.Build(), cancellationToken).ConfigureAwait(false);

            return statement;
        }
        catch
        {
            statement.Dispose();
            throw;
        }
    }

    private SnowflakeException MapUniversalCoreException(Exception ex, OperationScope scope, bool mapLastQuery = true) => ex switch
    {
        ServiceException serviceException => MapUniversalCoreError(serviceException, scope, mapLastQuery),
        TransportException transportException => MapUniversalCoreError(transportException, scope),
        OperationCanceledException => scope.ToCancellationException(ex, null, null),
        _ => throw new NotSupportedException($"Provided variant ({ex.GetType().Name}) is not supported!")
    };

    private static void EnrichException(SnowflakeException exception, QueryId queryId) => exception.QueryId ??= queryId.Value;

    private SnowflakeException MapUniversalCoreError(ServiceException exception, OperationScope scope, bool mapLastQuery)
    {
        var error = exception.Error;
        var queryId = error.HasQueryId ? error.QueryId : null;

        if (mapLastQuery)
            SetLastQueryId(queryId);

        // The ServiceException is not passed on as the cause, for the reason given on SnowflakeException.FromCore.
        return error.Kind == ErrorKind.Cancelled
            ? scope.ToCancellationException(null, error, queryId)
            : SnowflakeException.FromCore(exception);
    }

    private static SnowflakeException MapUniversalCoreError(TransportException exception, OperationScope scope) =>
        scope.Token.IsCancellationRequested
            ? scope.ToCancellationException(exception, null, null)
            : SnowflakeException.FromCore(exception);

    // Multi-statement responses are not supported yet: their result-set handles are not released and the
    // NotImplementedException reaches the caller unmapped until SNOW-4429197 adds them.
    private static ResultSetResponse GetSingleResult(ExecuteQueryResponse response) =>
        response.ResultCase == ExecuteQueryResponse.ResultOneofCase.Single
            ? response.Single
            : throw new NotImplementedException("Next milestone (SNOW-4429197), not yet implemented");

    private ConnectionHandle GetConnectionHandle() =>
        _connection.ConnHandle ?? throw new SnowflakeDriverException(SFError.EXECUTE_COMMAND_ON_CLOSED_CONNECTION, null);

    private void SetLastQueryId(string? queryId)
    {
        if (string.IsNullOrEmpty(queryId))
        {
            _logger.LogDebug("Trying to set empty last query id, skipping..");
            return;
        }

        _lastQueryId = queryId;
    }

    private OperationScope Register(OperationScope scope)
    {
        lock (_lock)
        {
            if (_inFlight is null)
            {
                _inFlight = scope;
                scope.Owner = this;
                return scope;
            }
        }

        scope.Dispose();
        throw new SnowflakeDriverException(SFError.STATEMENT_ALREADY_RUNNING_QUERY, null);
    }

    private void Unregister(OperationScope scope)
    {
        lock (_lock)
        {
            if (scope.Equals(_inFlight))
                _inFlight = null;
        }
    }

    /// <summary>
    /// The cancellation state of one operation: the token it observes, why that token may have been
    /// cancelled, and how a cancelled execution is reported to the caller.
    /// </summary>
    private sealed class OperationScope : IDisposable
    {
        private readonly CancellationTokenSource _cancellation;
        private readonly CancellationToken _callerToken;
        private readonly bool _isAsyncCaller;
        private readonly int _timeoutSeconds;
        private volatile bool _cancelRequested;

        private OperationScope(CancellationTokenSource cancellation, TimeSpan? timeout)
        {
            _cancellation = cancellation;
            if (timeout is not { } commandTimeout)
                return;

            _timeoutSeconds = (int)commandTimeout.TotalSeconds;
            _cancellation.CancelAfter(commandTimeout);
        }

        private OperationScope(TimeSpan? timeout, CancellationToken callerToken)
            : this(CancellationTokenSource.CreateLinkedTokenSource(callerToken), timeout)
        {
            _callerToken = callerToken;
            _isAsyncCaller = true;
        }

        internal SnowflakeStatement? Owner { get; set; }

        internal CancellationToken Token => _cancellation.Token;

        public void Dispose()
        {
            Owner?.Unregister(this);
            _cancellation.Dispose();
        }

        internal static OperationScope Sync(TimeSpan? timeout) =>
            new(new CancellationTokenSource(), timeout);

        internal static OperationScope Async(TimeSpan? timeout, CancellationToken callerToken) =>
            new(timeout, callerToken);

        /// <summary>Cancels the operation on behalf of <see cref="SnowflakeDbCommand.Cancel"/>.</summary>
        internal void RequestCancel()
        {
            _cancelRequested = true;
            try
            {
                _cancellation.Cancel();
            }
            catch (ObjectDisposedException)
            {
                // The operation completed concurrently; there is nothing left to cancel.
            }
        }

        /// <summary>
        /// Describes a cancelled execution in backward-compatible way: the caller's token gives
        /// QUERY_CANCELLED and an async timeout gives REQUEST_TIMEOUT, both as <see cref="OperationCanceledException"/>.
        /// <see cref="SnowflakeDbCommand.Cancel"/> and sync timeouts give the server's 604 when the abort
        /// reached a running query, and QUERY_CANCELLED otherwise.
        /// </summary>
        internal SnowflakeException ToCancellationException(Exception? cause, DriverException? coreError, string? queryId)
        {
            if (_callerToken.IsCancellationRequested)
                return new SnowflakeCancellationException(SFError.QUERY_CANCELLED, queryId, cause);

            if (_isAsyncCaller && !_cancelRequested && _timeoutSeconds > 0 && Token.IsCancellationRequested)
                return new SnowflakeCancellationException(SFError.REQUEST_TIMEOUT, queryId, cause, _timeoutSeconds);

            var abortReachedRunningQuery = coreError is { HasCancellationAbortOutcome: true, CancellationAbortOutcome: CancellationAbortOutcome.Aborted or CancellationAbortOutcome.NotConfirmed };
            return abortReachedRunningQuery
                ? new SnowflakeServerException(QueryCanceledSqlState.ToString(), QueryCanceledVendorCode, QueryCanceledMessage, queryId, cause)
                : new SnowflakeDriverException(SFError.QUERY_CANCELLED, queryId, cause);
        }
    }
}
