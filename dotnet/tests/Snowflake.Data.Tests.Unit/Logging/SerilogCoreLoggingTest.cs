namespace Snowflake.Data.Tests.Logging;

public sealed class SerilogCoreLoggingTest : LoggingProviderCoreScenarios
{
    public SerilogCoreLoggingTest(MockedLoginServer server)
        : base(server)
    {
    }

    protected override ILogSessionFactory Factory { get; } = new SerilogSessionFactory();
}
