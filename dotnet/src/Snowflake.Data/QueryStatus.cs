namespace Snowflake.Data;

/// <summary>
/// The status types of the query.
/// </summary>
public enum QueryStatus
{
    NoData,
    Running,
    Aborting,
    Success,
    FailedWithError,
    Aborted,
    Queued,
    FailedWithIncident,
    Disconnected,
    ResumingWarehouse,
    // purposeful typo
    QueuedReparingWarehouse,
    Restarted,
    Blocked,
}
