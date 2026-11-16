namespace Snowflake.Data.Tests.Logging;

public sealed class Log4NetCoreLoggingTest : LoggingProviderCoreScenarios
{
    public Log4NetCoreLoggingTest(MockedLoginServer server)
        : base(server)
    {
    }

    protected override ILogSessionFactory Factory { get; } = new Log4NetSessionFactory();
}
