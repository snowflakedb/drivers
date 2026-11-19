namespace Snowflake.Data.Tests.Logging;

public sealed class NLogCoreLoggingTest : LoggingProviderCoreScenarios
{
    public NLogCoreLoggingTest(MockedLoginServer server)
        : base(server)
    {
    }

    protected override ILogSessionFactory Factory { get; } = new NLogSessionFactory();
}
