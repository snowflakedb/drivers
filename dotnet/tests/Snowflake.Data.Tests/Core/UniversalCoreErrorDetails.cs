using Snowflake.Data.Proto;

namespace Snowflake.Data.Tests.Core;

/// <summary>
/// A core error whose fields other than the message, vendor code, SQL state and query ID all hold
/// <see cref="Marker"/>. Those fields appear in the message of a <see cref="ServiceException"/> built from it.
/// </summary>
internal static class UniversalCoreErrorDetails
{
    internal const string Marker = "core-detail-7f3a";

    internal static DriverException Error() => new()
    {
        Message = "SQL compilation error",
        ErrorTrace = { new ErrorTraceEntry { File = Marker, Line = 1, Column = 1, Message = Marker } },
        RootCause = Marker,
        RequestId = Marker,
        Parameter = Marker,
        ParameterValue = Marker,
    };
}
