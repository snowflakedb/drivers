using System.Diagnostics;
// ReSharper disable UnusedType.Global
// ReSharper disable UnusedMember.Global
// ReSharper disable UnusedParameter.Global

namespace Snowflake.Data;

public static class SnowflakeActivityStarter
{
    public const string ActivitySourceName = "Snowflake_dotnet_activity";
    public const string ClientDefinedTelemetrySourceName = "Client_custom_activity";

    public static Activity? StartActivity(this SnowflakeDbCommand command, string name)
    {
        // "TODO SNOW-4382577 this implementation is a stub for now. It awaits implementation (or dropping if applicable)");
        return null;
    }

    public static void SetSuccess(this Activity? activity)
    {
        // "TODO SNOW-4382577 this implementation is a stub for now. It awaits implementation (or dropping if applicable)");
    }

    public static void SetException(this Activity? activity, Exception? exception)
    {
        // "TODO SNOW-4382577 this implementation is a stub for now. It awaits implementation (or dropping if applicable)");
    }

    public static void AddTelemetryEvent(this Activity? activity, string name)
    {
        // "TODO SNOW-4382577 this implementation is a stub for now. It awaits implementation (or dropping if applicable)";
    }

    public static void AddTelemetryEvent(this Activity? activity, string name, ActivityTagsCollection? tags)
    {
        // "TODO SNOW-4382577 this implementation is a stub for now. It awaits implementation (or dropping if applicable)";
    }
}
