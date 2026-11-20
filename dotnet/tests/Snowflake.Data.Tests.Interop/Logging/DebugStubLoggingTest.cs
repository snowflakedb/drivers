using System.Diagnostics;
using Microsoft.Extensions.Logging;
using Snowflake.Data.Client;
using Snowflake.Data.Logging;
using Snowflake.Data.Tests.Logging;

namespace Snowflake.Data.Tests.Interop.Logging;

[Collection("Interop")]
[Trait("Category", "Interop")]
[Trait("Category", "Logging")]
public sealed class DebugStubLoggingTest : IDisposable
{
    private readonly ILogger _previous;

    public DebugStubLoggingTest()
    {
        _previous = CustomLoggerHolder.Current;
    }

    public void Dispose() => CustomLoggerHolder.Set(_previous);

    [SnowflakeFact]
    public void EmitsWithoutThrowingWhenDebuggerIsDetached()
    {
        Skip.When(Debugger.IsAttached, "DebugLogger writes to an attached debugger; this covers the detached case.");
        using var session = new DebugSessionFactory().Start(LogLevel.Trace);
        SnowflakeDbLoggerConfig.SetCustomLogger(session.Logger);

        Assert.Equal(0u, StubNativeMethods.sf_stub_emit_log(2, "info-event", "core.rs", 10, "emit", string.Empty));
        Assert.Equal(0u, StubNativeMethods.sf_stub_emit_log(4, "trace-event", "core.rs", 10, "emit", string.Empty));
        Assert.False(session.Logger.IsEnabled(LogLevel.Trace));
    }
}
