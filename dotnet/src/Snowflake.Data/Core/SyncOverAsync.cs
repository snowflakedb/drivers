namespace Snowflake.Data.Core;

/// <summary>
/// The single place where synchronous ADO.NET entry points block on the asynchronous implementation.
/// Every call into sf_core goes through its asynchronous FFI, because only that path can be
/// interrupted by <see cref="SnowflakeDbCommand.Cancel"/> or a command timeout.
/// </summary>
internal static class SyncOverAsync
{
    /// <summary>
    /// Blocks until <paramref name="task"/> completes and returns its result. A failure is rethrown
    /// as the original exception, not wrapped in an <see cref="AggregateException"/>.
    /// </summary>
    /// <remarks>
    /// Theoretically deadlocks should be virtually impossible because the asynchronous chain never captures a synchronization context
    /// (<c>ConfigureAwait(false)</c> throughout) and sf_core completes its tasks on its own threads.
    /// </remarks>
    internal static T Wait<T>(Task<T> task) =>
        // Sync-over-async: ADO.NET sync contract requires this
        task.GetAwaiter().GetResult();
}
