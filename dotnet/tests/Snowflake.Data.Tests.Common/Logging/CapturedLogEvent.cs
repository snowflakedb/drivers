using Microsoft.Extensions.Logging;

namespace Snowflake.Data.Tests.Logging;

public sealed record CapturedLogEvent(LogLevel Level, string Message);
