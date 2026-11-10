using Snowflake.Data.Core;
using Snowflake.Data.Core.Errors;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Interfaces;

/// <summary>
/// Looks up the server-side status of a query and waits for it to stop running. Failures are thrown
/// as <see cref="ServiceException"/> or <see cref="TransportException"/> from sf_core, or as
/// <see cref="SnowflakeDriverException"/>; the caller maps them.
/// </summary>
internal interface IQueryStatusPoller
{
    /// <summary>Looks up the status of <paramref name="queryId"/> once.</summary>
    /// <exception cref="SnowflakeDriverException"><see cref="SFError.UNKNOWN_QUERY_STATUS"/> for a status name the driver does not know.</exception>
    Task<QueryStatus> FetchStatusAsync(ConnectionHandle connection, QueryId queryId, CancellationToken cancellationToken);

    /// <summary>
    /// Polls <paramref name="queryId"/> until its status is no longer a running one; a failed query
    /// also ends the wait. <paramref name="getConnection"/> is called before every poll, so an exception
    /// it throws ends the wait.
    /// </summary>
    /// <exception cref="SnowflakeDriverException">
    /// <see cref="SFError.QUERY_RESULT_NO_DATA_RETRIES_EXCEEDED"/> when the server answers NO_DATA more often than the schedule tolerates.
    /// </exception>
    Task WaitUntilFinishedAsync(Func<ConnectionHandle> getConnection, QueryId queryId, CancellationToken cancellationToken);
}
