namespace Snowflake.Data.Tests.Logging;

public sealed class EventSourceCoreLoggingTest : LoggingProviderCoreScenarios
{
    public EventSourceCoreLoggingTest(MockedLoginServer server)
        : base(server)
    {
    }

    protected override ILogSessionFactory Factory { get; } = new EventSourceSessionFactory();
}
