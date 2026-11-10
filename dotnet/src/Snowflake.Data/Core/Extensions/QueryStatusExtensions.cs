namespace Snowflake.Data.Core.Extensions;

/// <summary>
/// Maps the status names the server reports onto <see cref="QueryStatus"/> and classifies them
/// in a backward-compatible way.
/// </summary>
internal static class QueryStatusExtensions
{
    private static readonly Dictionary<string, QueryStatus> StatusesByServerName = new(StringComparer.OrdinalIgnoreCase)
    {
        ["NO_DATA"] = QueryStatus.NoData,
        ["RUNNING"] = QueryStatus.Running,
        ["ABORTING"] = QueryStatus.Aborting,
        ["SUCCESS"] = QueryStatus.Success,
        ["FAILED_WITH_ERROR"] = QueryStatus.FailedWithError,
        ["ABORTED"] = QueryStatus.Aborted,
        ["QUEUED"] = QueryStatus.Queued,
        ["FAILED_WITH_INCIDENT"] = QueryStatus.FailedWithIncident,
        ["DISCONNECTED"] = QueryStatus.Disconnected,
        ["RESUMING_WAREHOUSE"] = QueryStatus.ResumingWarehouse,
        // purposeful typo, it is what the server sends
        ["QUEUED_REPARING_WAREHOUSE"] = QueryStatus.QueuedReparingWarehouse,
        ["RESTARTED"] = QueryStatus.Restarted,
        ["BLOCKED"] = QueryStatus.Blocked,
    };

    extension(QueryStatus status)
    {
        /// <summary>Whether a query in <paramref name="status"/> has not finished executing yet.</summary>
        internal bool IsStillRunning() => status switch
        {
            QueryStatus.Running => true,
            QueryStatus.ResumingWarehouse => true,
            QueryStatus.Queued => true,
            QueryStatus.QueuedReparingWarehouse => true,
            QueryStatus.NoData => true,
            _ => false,
        };

        /// <summary>Whether a query in <paramref name="status"/> has failed.</summary>
        internal bool IsAnError() => status switch
        {
            QueryStatus.Aborting => true,
            QueryStatus.FailedWithError => true,
            QueryStatus.Aborted => true,
            QueryStatus.FailedWithIncident => true,
            QueryStatus.Disconnected => true,
            QueryStatus.Blocked => true,
            _ => false,
        };
    }

    /// <summary>Resolves a status name reported by the server, case-insensitively.</summary>
    internal static bool TryParseServerName(string serverName, out QueryStatus status) =>
        StatusesByServerName.TryGetValue(serverName, out status);
}
