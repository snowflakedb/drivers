// ZLoggerSessionFactory exists only on modern .NET; the ZLogger package is not referenced for .NET Framework.
#if !NETFRAMEWORK
namespace Snowflake.Data.Tests.Logging;

public sealed class ZLoggerCoreLoggingTest : LoggingProviderCoreScenarios
{
    public ZLoggerCoreLoggingTest(MockedLoginServer server)
        : base(server)
    {
    }

    protected override ILogSessionFactory Factory { get; } = new ZLoggerSessionFactory();
}
#endif
