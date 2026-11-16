using Microsoft.Extensions.Logging;

namespace Snowflake.Data.Tests.Logging;

public interface ILogSessionFactory
{
    ICapturedLogSession Start(LogLevel minimum);
}
