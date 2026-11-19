using Microsoft.Extensions.Logging;

namespace Snowflake.Data.Tests.ArchitectureInvariantTests.Dummies;

// do not move/remove/rename/modify this.
// Violation: EventIdBase shared with OverlappingEventIdDummyLog; the method EventIds are valid
internal static partial class ValidEventIdDummyLog
{
    private const int EventIdBase = 1000;

    [LoggerMessage(EventId = EventIdBase + 1, Level = LogLevel.Debug, Message = "First")]
    public static partial void First(this ILogger logger);

    [LoggerMessage(EventId = EventIdBase + 2, Level = LogLevel.Debug, Message = "Second")]
    public static partial void Second(this ILogger logger);
}

// Violation: EventIdBase shared with ValidEventIdDummyLog
internal static partial class OverlappingEventIdDummyLog
{
    private const int EventIdBase = 1000;

    [LoggerMessage(EventId = EventIdBase + 3, Level = LogLevel.Debug, Message = "Overlapping")]
    public static partial void Overlapping(this ILogger logger);
}

internal static partial class InvalidEventIdDummyLog
{
    private const int EventIdBase = 2000;

    // Violation: EventId not set
    [LoggerMessage(Level = LogLevel.Debug, Message = "Missing")]
    public static partial void Missing(this ILogger logger);

    // Violation: EventId does not use EventIdBase
    [LoggerMessage(EventId = 2002, Level = LogLevel.Debug, Message = "HardCoded")]
    public static partial void HardCoded(this ILogger logger);

    // Violation: offset outside 1..999
    [LoggerMessage(EventId = EventIdBase + 1000, Level = LogLevel.Debug, Message = "OutOfRange")]
    public static partial void OutOfRange(this ILogger logger);

    [LoggerMessage(EventId = EventIdBase + 1, Level = LogLevel.Debug, Message = "Unique")]
    public static partial void Unique(this ILogger logger);

    // Violation: offset already used by Unique
    [LoggerMessage(EventId = EventIdBase + 1, Level = LogLevel.Debug, Message = "Duplicate")]
    public static partial void Duplicate(this ILogger logger);
}

// Violation: EventIdBase is not a multiple of 1000
internal static partial class MisalignedEventIdDummyLog
{
    private const int EventIdBase = 4500;

    [LoggerMessage(EventId = EventIdBase + 1, Level = LogLevel.Debug, Message = "Misaligned")]
    public static partial void Misaligned(this ILogger logger);
}

// Violation: no EventIdBase constant
internal static partial class MissingBaseEventIdDummyLog
{
    [LoggerMessage(EventId = 5001, Level = LogLevel.Debug, Message = "NoBase")]
    public static partial void NoBase(this ILogger logger);
}
