// ZLoggerSessionFactory exists only on modern .NET; the ZLogger package is not referenced for .NET Framework.
#if !NETFRAMEWORK
using Snowflake.Data.Tests.Logging;

namespace Snowflake.Data.Tests.Interop.Logging;

public sealed class ZLoggerStubLoggingTest : LoggingProviderStubScenarios
{
    protected override ILogSessionFactory Factory { get; } = new ZLoggerSessionFactory();
}
#endif
