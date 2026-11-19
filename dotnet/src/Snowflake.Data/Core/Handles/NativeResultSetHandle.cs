using Microsoft.Extensions.Logging;
using Snowflake.Data.Core.Extensions;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Core.Handles;

/// <summary>
/// Owns one sf_core result set handle and releases it on <see cref="Dispose"/>.
/// </summary>
/// <remarks>
/// <para>
/// The release is synchronous: it runs once the result's Arrow C stream is open, and from
/// operations whose token is already cancelled, and it only drops an entry from sf_core's in-memory
/// handle table. A failed release is logged and does not propagate, so it cannot replace the
/// exception of the operation that owned the handle.
/// </para>
/// <para>
/// Every owner disposes the handle through a <c>using</c> scope, or creates it in a factory that disposes it on
/// failure and hands it to a <c>using</c> scope. There is no finalizer: the handle holds no .NET memory, and a
/// handle that is never disposed belongs to an operation that never resumed, which keeps the handle reachable.
/// </para>
/// </remarks>
internal sealed class NativeResultSetHandle : IDisposable
{
    private readonly IDatabaseDriverService _driverService;
    private readonly ILogger _logger;
    private int _released;

    internal NativeResultSetHandle(IDatabaseDriverService driverService, ResultSetHandle handle, ILogger logger)
    {
        _driverService = driverService;
        Handle = handle;
        _logger = logger;
    }

    internal ResultSetHandle Handle { get; }

    public void Dispose()
    {
        if (Interlocked.Exchange(ref _released, 1) != 0)
            return;

        try
        {
            _driverService.ResultSetRelease(this.BuildReleaseRequest());
        }
        catch (Exception ex) when (ex is ServiceException or TransportException)
        {
            _logger.LogCritical(ex, "Failed to release the result set handle");
        }
    }
}
