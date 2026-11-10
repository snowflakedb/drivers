using System.Data;
using System.Data.Common;
using System.Diagnostics;
using System.Diagnostics.CodeAnalysis;
using Microsoft.Extensions.Logging;
using Snowflake.Data.Core;
using Snowflake.Data.Core.Errors;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Telemetry;

namespace Snowflake.Data;

/// <summary>
/// Represents a SQL statement to execute against Snowflake.
/// </summary>
/// <remarks>
/// A command is not thread-safe. Run one operation at a time; the command can be reused once the
/// operation has completed, and starting another operation while one is still running fails with
/// <see cref="SFError.STATEMENT_ALREADY_RUNNING_QUERY"/>. <see cref="Cancel"/> is the exception:
/// it may be called from any thread to stop the running operation.
/// </remarks>
public sealed class SnowflakeDbCommand : DbCommand
{
    /// <summary>0</summary>
    private const int CommandTimeoutDefaultValue = 0;

    private readonly ISnowflakeStatement? _overrideStatement;
    private readonly ILogger<SnowflakeDbCommand> _logger;

    private volatile ISnowflakeStatement? _statement;

    public SnowflakeDbCommand() : this(null, LoggerAndTelemetryStubs.GetLogger<SnowflakeDbCommand>())
    {
    }

    public SnowflakeDbCommand(SnowflakeDbConnection connection) : this()
    {
        Connection = connection;
    }

    public SnowflakeDbCommand(SnowflakeDbConnection connection, string cmdText) : this(connection)
    {
        CommandText = cmdText;
    }

    internal SnowflakeDbCommand(ISnowflakeStatement? statement, ILogger<SnowflakeDbCommand> logger)
    {
        logger.LogDebug("Constructing {DbCommandClass} class", nameof(SnowflakeDbCommand));
        _overrideStatement = statement;
        _logger = logger;
    }

    [AllowNull]
    public override string CommandText { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the client-side time limit, in seconds, for the whole call: executing the command, or
    /// waiting for and fetching the results in <see cref="GetResultsFromQueryId"/>. When it expires during
    /// an execution the query is aborted on the server; when it expires during
    /// <see cref="GetResultsFromQueryId"/> the call stops and the query is left as it is on the server.
    /// 0 or a negative value means no limit.
    /// Default is <inheritdoc cref="CommandTimeoutDefaultValue"/>.
    /// </summary>
    public override int CommandTimeout { get; set; } = CommandTimeoutDefaultValue;

    /// <summary>
    /// Gets or sets the QUERY_TAG for the queries this command executes. It overrides the connection's
    /// query tag for those queries only; null keeps the connection's tag.
    /// </summary>
    public string? QueryTag { get; set; }

    public override CommandType CommandType
    {
        get => CommandType.Text;

        set
        {
            if (value != CommandType.Text)
                throw new SnowflakeDbException(SFError.UNSUPPORTED_FEATURE);
        }
    }

    public override bool DesignTimeVisible
    {
        get => false;
        set
        {
            if (value)
                throw new SnowflakeDbException(SFError.UNSUPPORTED_FEATURE);
        }
    }

    public override UpdateRowSource UpdatedRowSource
    {
        get => UpdateRowSource.None;

        set
        {
            if (value != UpdateRowSource.None)
                throw new SnowflakeDbException(SFError.UNSUPPORTED_FEATURE);
        }
    }

    protected override DbConnection? DbConnection
    {
        get;

        set
        {
            if (value == null)
            {
                if (field == null)
                    return;

                // Unsetting connection not supported.
                throw new SnowflakeDbException(SFError.UNSUPPORTED_FEATURE);
            }

            // Must be of type SnowflakeDbConnection.
            if (value is not SnowflakeDbConnection sfc)
                throw new SnowflakeDbException(SFError.UNSUPPORTED_FEATURE);

            // Resetting connection not supported.
            if (field != null && field != sfc)
                throw new SnowflakeDbException(SFError.UNSUPPORTED_FEATURE);

            field = sfc;
        }
    }

    protected override DbParameterCollection DbParameterCollection { get; } =
        new SnowflakeDbParameterCollection();

    protected override DbTransaction? DbTransaction { get; set; }

    /// <summary>
    /// Attempts to cancel the operation this command is running. An execution also has its query aborted
    /// on the server; <see cref="GetResultsFromQueryId"/> stops without affecting the query on the server.
    /// Does nothing when no operation is running. May be called from any thread.
    /// </summary>
    public override void Cancel()
    {
        _logger.LogDebug("Executing {Method}", nameof(Cancel));
        using var activity = LoggerAndTelemetryStubs.StartActivity(TelemetryActivities.CommandCancel);

        // Doesn't create a statement: there is nothing to cancel before the first execution.
        _statement?.Cancel();
        activity?.SetSuccess();
    }

    /// <inheritdoc />
    public override int ExecuteNonQuery() =>
        Run(nameof(ExecuteNonQuery), TelemetryActivities.ExecuteNonQuery, activity =>
        {
            var ctx = GetStatementCtx(describeOnly: false);
            return GetStatement().ExecuteNonQuery(ctx, activity);
        });

    /// <inheritdoc />
    public override Task<int> ExecuteNonQueryAsync(CancellationToken cancellationToken) =>
        RunAsync(nameof(ExecuteNonQueryAsync), TelemetryActivities.ExecuteNonQueryAsync, (activity, token) =>
        {
            token.ThrowIfCancellationRequested();
            var ctx = GetStatementCtx(describeOnly: false);
            return GetStatement().ExecuteNonQueryAsync(ctx, activity, token);
        }, cancellationToken);

    /// <summary>
    /// Executes the query and returns the first column of the first row, or <see cref="DBNull.Value"/>
    /// when the result has no rows.
    /// </summary>
    public override object? ExecuteScalar() =>
        Run<object?>(nameof(ExecuteScalar), TelemetryActivities.ExecuteScalar, _ =>
        {
            var ctx = GetStatementCtx(describeOnly: false);
            using var reader = GetStatement().ExecuteReader(ctx);
            return reader.Read() ? reader.GetValue(0) : DBNull.Value;
        });

    /// <summary>
    /// Executes the query and returns the first column of the first row, or <see cref="DBNull.Value"/>
    /// when the result has no rows.
    /// </summary>
    public override Task<object?> ExecuteScalarAsync(CancellationToken cancellationToken) =>
        RunAsync<object?>(nameof(ExecuteScalarAsync), TelemetryActivities.ExecuteScalarAsync, async (_, token) =>
        {
            token.ThrowIfCancellationRequested();
            var ctx = GetStatementCtx(describeOnly: false);
            using var reader = await GetStatement().ExecuteReaderAsync(ctx, token).ConfigureAwait(false);
            return await reader.ReadAsync(token).ConfigureAwait(false) ? reader.GetValue(0) : DBNull.Value;
        }, cancellationToken);

    /// <summary>
    /// Executes a PUT command uploading data from the provided in-memory stream instead of a local file.
    /// The caller is responsible for disposing the stream and ensuring thread-safe access to it.
    /// </summary>
    /// <returns>A <see cref="DbDataReader"/> containing the upload result metadata.</returns>
    public DbDataReader ExecuteDbDataReaderWithMemoryStream(MemoryStream stream) =>
        throw new NotImplementedException("Future milestone (SNOW-4429445), not yet implemented");

    /// <summary>
    /// Execute a query in async mode.
    /// Async mode means the server will respond immediately with the query ID and execute the query asynchronously
    /// </summary>
    /// <returns>The query id.</returns>
    public string ExecuteInAsyncMode() =>
        Run(nameof(ExecuteInAsyncMode), TelemetryActivities.ExecuteInAsyncMode, _ =>
        {
            var ctx = GetStatementCtx(describeOnly: false);
            return GetStatement().ExecuteInAsyncMode(ctx);
        });

    /// <summary>
    /// Executes an asynchronous query in async mode.
    /// Async mode means the server will respond immediately with the query ID and execute the query asynchronously
    /// </summary>
    /// <param name="cancellationToken"></param>
    /// <returns>The query id.</returns>
    public Task<string> ExecuteAsyncInAsyncMode(CancellationToken cancellationToken) =>
        RunAsync(nameof(ExecuteAsyncInAsyncMode), TelemetryActivities.ExecuteAsyncInAsyncMode, (_, token) =>
        {
            var ctx = GetStatementCtx(describeOnly: false);
            return GetStatement().ExecuteInAsyncModeAsync(ctx, token);
        }, cancellationToken);

    /// <summary>
    /// Gets the query status based on query ID.
    /// </summary>
    /// <param name="queryId"></param>
    /// <returns>The query status.</returns>
    public QueryStatus GetQueryStatus(string queryId)
    {
        var parsedQueryId = ParseQueryId(queryId);
        return Run(nameof(GetQueryStatus), TelemetryActivities.GetQueryStatus,
            _ => GetStatement().GetQueryStatus(parsedQueryId));
    }

    /// <summary>
    /// Gets the query status based on query ID.
    /// </summary>
    /// <param name="queryId"></param>
    /// <param name="cancellationToken"></param>
    /// <returns>The query status.</returns>
    public Task<QueryStatus> GetQueryStatusAsync(string queryId, CancellationToken cancellationToken)
    {
        var parsedQueryId = ParseQueryId(queryId);
        return RunAsync(nameof(GetQueryStatusAsync), TelemetryActivities.GetQueryStatusAsync,
            (_, token) => GetStatement().GetQueryStatusAsync(parsedQueryId, token), cancellationToken);
    }

    /// <summary>
    /// Gets the query results based on query ID, waiting for the query to finish first. The wait and the
    /// fetch of the results together are limited by <see cref="CommandTimeout"/>. Stopping the call,
    /// through <see cref="Cancel"/> or the timeout, sends no abort: a running query keeps running and
    /// the results of a finished one stay on the server.
    /// </summary>
    /// <param name="queryId"></param>
    /// <returns>The query results.</returns>
    public DbDataReader GetResultsFromQueryId(string queryId)
    {
        var parsedQueryId = ParseQueryId(queryId);
        return Run<DbDataReader>(nameof(GetResultsFromQueryId), TelemetryActivities.GetResultsFromQueryId,
            _ => GetStatement().GetResultsFromQueryId(parsedQueryId, GetTimeout()));
    }

    /// <summary>
    /// Gets the query results based on query ID, waiting for the query to finish first. The wait and the
    /// fetch of the results together are limited by <see cref="CommandTimeout"/>. Stopping the call,
    /// through <see cref="Cancel"/>, the cancellation token or the timeout, sends no abort: a running
    /// query keeps running and the results of a finished one stay on the server.
    /// </summary>
    /// <param name="queryId"></param>
    /// <param name="cancellationToken"></param>
    /// <returns>The query results.</returns>
    public Task<DbDataReader> GetResultsFromQueryIdAsync(string queryId, CancellationToken cancellationToken)
    {
        var parsedQueryId = ParseQueryId(queryId);
        return RunAsync<DbDataReader>(nameof(GetResultsFromQueryIdAsync),
            TelemetryActivities.GetResultsFromQueryIdAsync,
            async (_, token) => await GetStatement().GetResultsFromQueryIdAsync(parsedQueryId, GetTimeout(), token)
                .ConfigureAwait(false), cancellationToken);
    }

    /// <summary>
    /// Prepares the command for execution.
    /// This method is currently not implemented and acts as a no-operation (Noop).
    /// </summary>
    public override void Prepare()
    { }

    /// <summary>
    /// Gets the Snowflake query ID of the last query this command executed, fetched or failed on,
    /// or null when it has not run one yet.
    /// </summary>
    public string? GetQueryId() => _statement?.LastQueryId;

    protected override DbParameter CreateDbParameter() =>
        new SnowflakeDbParameter();

    protected override DbDataReader ExecuteDbDataReader(CommandBehavior behavior) =>
        Run<DbDataReader>(nameof(ExecuteDbDataReader), TelemetryActivities.ExecuteDbDataReader, _ =>
        {
            var ctx = GetStatementCtx(IsSchemaOnly(behavior));
            return GetStatement().ExecuteReader(ctx);
        });

    protected override Task<DbDataReader> ExecuteDbDataReaderAsync(CommandBehavior behavior, CancellationToken cancellationToken) =>
        RunAsync<DbDataReader>(nameof(ExecuteDbDataReaderAsync), TelemetryActivities.ExecuteDbDataReaderAsync, async (_, token) =>
        {
            var ctx = GetStatementCtx(IsSchemaOnly(behavior));
            return await GetStatement().ExecuteReaderAsync(ctx, token).ConfigureAwait(false);
        }, cancellationToken);

    protected override void Dispose(bool disposing)
    {
        if (disposing)
            _statement?.Dispose();

        base.Dispose(disposing);
    }

    private T Run<T>(string method, string activityName, Func<Activity?, T> operation)
    {
        _logger.LogDebug("Executing {Method}", method);
        using var activity = LoggerAndTelemetryStubs.StartActivity(activityName);
        try
        {
            var result = operation(activity);
            activity?.SetSuccess();
            return result;
        }
        catch (SnowflakeException carrier)
        {
            throw ToPublicException(carrier, method, activity);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "The command {Method} failed to execute.", method);
            activity?.SetException(ex);
            throw;
        }
    }

    private async Task<T> RunAsync<T>(string method, string activityName, Func<Activity?, CancellationToken, Task<T>> operation, CancellationToken cancellationToken)
    {
        _logger.LogDebug("Executing {Method}", method);
        using var activity = LoggerAndTelemetryStubs.StartActivity(activityName);
        try
        {
            var result = await operation(activity, cancellationToken).ConfigureAwait(false);
            activity?.SetSuccess();
            return result;
        }
        catch (SnowflakeException carrier)
        {
            throw ToPublicException(carrier, method, activity);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "The command {Method} failed to execute.", method);
            activity?.SetException(ex);
            throw;
        }
    }

    // Built in the catch block rather than an exception filter: an exception thrown inside a filter
    // is swallowed, which would let the internal carrier escape to the caller.
    private Exception ToPublicException(SnowflakeException carrier, string method, Activity? activity)
    {
        _logger.LogError(carrier, "The command {Method} failed to execute.", method);
        // TODO SNOW-4382591 next milestone: every SnowflakeDbException constructor throws NotImplementedException until
        // the public exception gets its real implementation, so today this line throws instead of returning.
        var exception = carrier.ToPublicException();
        activity?.SetException(exception);
        return exception;
    }

    private SnowflakeStatementCtx GetStatementCtx(bool describeOnly)
    {
        if (string.IsNullOrEmpty(CommandText))
            throw new SnowflakeDriverException(SFError.COMMAND_TEXT_NOT_SET, null);

        if (DbParameterCollection.Count > 0)
            throw new NotImplementedException("Future milestone (SNOW-4382583), not yet implemented");

        return new SnowflakeStatementCtx(CommandText, QueryTag, GetTimeout(), describeOnly);
    }

    private TimeSpan? GetTimeout() => CommandTimeout > 0 ? TimeSpan.FromSeconds(CommandTimeout) : null;

    private ISnowflakeStatement GetStatement()
    {
        if (DbConnection is not SnowflakeDbConnection { State: ConnectionState.Open } connection || ((IConnectionHandleWrapper)connection).ConnHandle == null)
            throw new SnowflakeDriverException(SFError.EXECUTE_COMMAND_ON_CLOSED_CONNECTION, null);

        // Not atomic: the command is not thread-safe. Two threads racing the first execution can each build a
        // statement and see different instances; that torn read is acceptable for a misused command.
        return _statement ??= _overrideStatement ?? new SnowflakeStatement(connection);
    }

    // Throws before Run/RunAsync, so the async lookups fail synchronously rather than with a faulted task.
    private static QueryId ParseQueryId(string? queryId) =>
        QueryId.TryParse(queryId, out var parsed)
            ? parsed
            : throw new SnowflakeDbException(SFError.INVALID_QUERY_ID_FORMAT);

    private static bool IsSchemaOnly(CommandBehavior behavior) =>
        (behavior & CommandBehavior.SchemaOnly) == CommandBehavior.SchemaOnly;
}
