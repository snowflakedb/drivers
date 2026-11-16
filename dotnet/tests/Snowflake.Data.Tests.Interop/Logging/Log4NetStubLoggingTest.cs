using Snowflake.Data.Tests.Logging;

namespace Snowflake.Data.Tests.Interop.Logging;

public sealed class Log4NetStubLoggingTest : LoggingProviderStubScenarios
{
    protected override ILogSessionFactory Factory { get; } = new Log4NetSessionFactory();
}
