using Microsoft.Extensions.Logging;
using Snowflake.Data.Core.Extensions;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Core.Handles;

/// <summary>
/// Owns one sf_core statement handle and releases it on <see cref="Dispose"/>.
/// </summary>
/// <remarks>
/// The release is synchronous: it has to run even when the operation's token is already cancelled,
/// and it only drops an entry from sf_core's in-memory handle table. A failed release is logged and
/// does not propagate, so it cannot replace the exception to the operation that owned the handle.
/// </remarks>
internal sealed class NativeStatementHandle : IDisposable
{
    private readonly IDatabaseDriverService _driverService;
    private readonly ILogger _logger;
    private int _released;

    internal NativeStatementHandle(IDatabaseDriverService driverService, StatementHandle handle, ILogger logger)
    {
        _driverService = driverService;
        Handle = handle;
        _logger = logger;
    }

    internal StatementHandle Handle { get; }

    public void Dispose()
    {
        if (Interlocked.Exchange(ref _released, 1) != 0)
            return;

        try
        {
            _driverService.StatementRelease(this.BuildReleaseRequest());
        }
        catch (Exception ex) when (ex is ServiceException or TransportException)
        {
            _logger.LogCritical(ex, "Failed to release the statement handle");
        }
    }
}
