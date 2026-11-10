using Snowflake.Data.Core.Errors;
using Snowflake.Data.Core.Extensions;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Core;

/// <summary>
/// Polls sf_core for query status on the delays of a <see cref="QueryResultsRetryConfig"/>.
/// </summary>
internal sealed class QueryStatusPoller : IQueryStatusPoller
{
    private readonly IDatabaseDriverService _driverService;
    private readonly QueryResultsRetryConfig _retryConfig;

    /// <summary>Creates a poller that sends its lookups through <paramref name="driverService"/>.</summary>
    internal QueryStatusPoller(IDatabaseDriverService driverService, QueryResultsRetryConfig retryConfig)
    {
        _driverService = driverService;
        _retryConfig = retryConfig;
    }

    /// <inheritdoc />
    public async Task<QueryStatus> FetchStatusAsync(ConnectionHandle connection, QueryId queryId, CancellationToken cancellationToken)
    {
        var request = new ConnectionGetQueryStatusRequest { ConnHandle = connection, QueryId = queryId.Value };
        var response = await _driverService.ConnectionGetQueryStatusAsync(request, cancellationToken).ConfigureAwait(false);
        return QueryStatusExtensions.TryParseServerName(response.StatusName, out var status)
            ? status
            : throw new SnowflakeDriverException(SFError.UNKNOWN_QUERY_STATUS, queryId.Value);
    }

    /// <inheritdoc />
    public async Task WaitUntilFinishedAsync(Func<ConnectionHandle> getConnection, QueryId queryId, CancellationToken cancellationToken)
    {
        var noDataCount = 0;
        foreach (var retryPattern in _retryConfig.RetryPattern)
        {
            cancellationToken.ThrowIfCancellationRequested();

            var status = await FetchStatusAsync(getConnection.Invoke(), queryId, cancellationToken).ConfigureAwait(false);
            if (!status.IsStillRunning())
                return;

            await Task.Delay(retryPattern, cancellationToken).ConfigureAwait(false);

            if (status == QueryStatus.NoData && ++noDataCount > _retryConfig.NoDataMaxRetries)
                throw new SnowflakeDriverException(SFError.QUERY_RESULT_NO_DATA_RETRIES_EXCEEDED, queryId.Value);
        }
    }
}
