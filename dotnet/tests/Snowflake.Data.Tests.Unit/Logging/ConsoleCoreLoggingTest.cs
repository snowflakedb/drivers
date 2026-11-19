namespace Snowflake.Data.Tests.Logging;

public sealed class ConsoleCoreLoggingTest : LoggingProviderCoreScenarios
{
    public ConsoleCoreLoggingTest(MockedLoginServer server)
        : base(server)
    {
    }

    protected override ILogSessionFactory Factory { get; } = new ConsoleSessionFactory();
}
