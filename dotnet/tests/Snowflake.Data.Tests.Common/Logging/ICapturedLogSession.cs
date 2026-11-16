using Microsoft.Extensions.Logging;

namespace Snowflake.Data.Tests.Logging;

public interface ICapturedLogSession : IDisposable
{
    ILogger Logger { get; }

    /// <summary>
    /// Returns the events captured so far. Some sessions shut their provider down to flush it,
    /// so events logged after this call may not be captured.
    /// </summary>
    IReadOnlyList<CapturedLogEvent> GetEvents();
}
