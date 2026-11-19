using System.Diagnostics;
using Microsoft.Extensions.Logging;
using Snowflake.Data.Logging;
using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests.Logging;

[Collection("CustomLogger")]
[Trait("Category", "Interop")]
[Trait("Category", "Logging")]
public sealed class DebugCoreLoggingTest : IDisposable
{
    private readonly MockedLoginServer _server;
    private readonly ILogger _previous;

    public DebugCoreLoggingTest(MockedLoginServer server)
    {
        _server = server;
        _previous = CustomLoggerHolder.Current;
    }

    public void Dispose() => CustomLoggerHolder.Set(_previous);

    [SnowflakeFact]
    public async Task OpenDoesNotThrowWhenDebuggerIsDetachedAsync()
    {
        Skip.When(Debugger.IsAttached, "DebugLogger writes to an attached debugger; this covers the detached case.");
        using var session = new DebugSessionFactory().Start(LogLevel.Trace);
        SnowflakeDbLoggerConfig.SetCustomLogger(session.Logger);

        await _server.OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.False(session.Logger.IsEnabled(LogLevel.Trace));
    }
}
