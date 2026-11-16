using Snowflake.Data.Tests.Logging;

namespace Snowflake.Data.Tests.Interop.Logging;

public sealed class SerilogStubLoggingTest : LoggingProviderStubScenarios
{
    protected override ILogSessionFactory Factory { get; } = new SerilogSessionFactory();
}
