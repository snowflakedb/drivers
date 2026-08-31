using System.Diagnostics;
// ReSharper disable UnusedType.Global
// ReSharper disable UnusedMember.Global
// ReSharper disable UnusedParameter.Global

namespace Snowflake.Data;

public static class SnowflakeActivityStarter
{
    public const string ActivitySourceName = "Snowflake_dotnet_activity";
    public const string ClientDefinedTelemetrySourceName = "Client_custom_activity";

    public static Activity? StartActivity(this SnowflakeDbCommand command, string name) => throw new NotImplementedException(
        "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");

    public static void SetSuccess(this Activity? activity) => throw new NotImplementedException(
        "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");

    public static void SetException(this Activity? activity, Exception? exception) => throw new NotImplementedException(
        "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");

    public static void AddTelemetryEvent(this Activity? activity, string name) => throw new NotImplementedException(
        "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");

    public static void AddTelemetryEvent(this Activity? activity, string name, ActivityTagsCollection? tags) => throw new NotImplementedException(
        "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");
}
