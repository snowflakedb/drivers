using Snowflake.Data.Tests.Logging;

namespace Snowflake.Data.Tests.Interop.Logging;

public sealed class EventSourceStubLoggingTest : LoggingProviderStubScenarios
{
    protected override ILogSessionFactory Factory { get; } = new EventSourceSessionFactory();
}
