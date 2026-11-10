using System.Data.Common;
using System.Diagnostics;
using Snowflake.Data.Core;
using Snowflake.Data.Core.Errors;

namespace Snowflake.Data.Interfaces;

/// <summary>
/// Runs the operations of one <see cref="SnowflakeDbCommand"/>. Each execution uses its own sf_core
/// statement handle, at most one execution or result wait is in flight at a time, and
/// <see cref="Cancel"/> may be called from any thread. Failures are thrown as
/// <see cref="SnowflakeException"/>, which the command converts at its public boundary.
/// </summary>
internal interface ISnowflakeStatement : IDisposable
{
    /// <summary>The query ID of the last query executed, fetched or failed through this statement, or null.</summary>
    string? LastQueryId { get; }

    /// <summary>Cancels the operation in flight, if any. Never throws.</summary>
    void Cancel();

    /// <summary>Executes <paramref name="ctx"/> and returns the rows-affected count.</summary>
    int ExecuteNonQuery(SnowflakeStatementCtx ctx, Activity? activity);

    /// <summary>Executes <paramref name="ctx"/> and returns the rows-affected count.</summary>
    Task<int> ExecuteNonQueryAsync(SnowflakeStatementCtx ctx, Activity? activity, CancellationToken cancellationToken);

    /// <summary>Executes <paramref name="ctx"/> (describe-only when requested) and opens a reader over the result.</summary>
    DbDataReader ExecuteReader(SnowflakeStatementCtx ctx);

    /// <summary>Executes <paramref name="ctx"/> (describe-only when requested) and opens a reader over the result.</summary>
    Task<DbDataReader> ExecuteReaderAsync(SnowflakeStatementCtx ctx, CancellationToken cancellationToken);

    /// <summary>Submits <paramref name="ctx"/> without waiting for it to finish and returns its query ID.</summary>
    string ExecuteInAsyncMode(SnowflakeStatementCtx ctx);

    /// <summary>Submits <paramref name="ctx"/> without waiting for it to finish and returns its query ID.</summary>
    Task<string> ExecuteInAsyncModeAsync(SnowflakeStatementCtx ctx, CancellationToken cancellationToken);

    /// <summary>Looks up the status of the query <paramref name="queryId"/>.</summary>
    QueryStatus GetQueryStatus(QueryId queryId);

    /// <summary>Looks up the status of the query <paramref name="queryId"/>.</summary>
    Task<QueryStatus> GetQueryStatusAsync(QueryId queryId, CancellationToken cancellationToken);

    /// <summary>
    /// Waits for the query <paramref name="queryId"/> to finish and opens a reader over its result.
    /// The wait and the fetch together are limited to <paramref name="timeout"/>; null means no limit.
    /// </summary>
    DbDataReader GetResultsFromQueryId(QueryId queryId, TimeSpan? timeout);

    /// <summary>
    /// Waits for the query <paramref name="queryId"/> to finish and opens a reader over its result.
    /// The wait and the fetch together are limited to <paramref name="timeout"/>; null means no limit.
    /// </summary>
    Task<DbDataReader> GetResultsFromQueryIdAsync(QueryId queryId, TimeSpan? timeout, CancellationToken cancellationToken);
}
