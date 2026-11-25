extern alias OldDriver;
using System.Reflection;
using Snowflake.Data.Tests.Compatibility;
using OldSnowflakeDbConnection = OldDriver::Snowflake.Data.Client.SnowflakeDbConnection;
using OldSnowflakeDbConnectionPool = OldDriver::Snowflake.Data.Client.SnowflakeDbConnectionPool;

namespace Snowflake.Data.Tests.E2E;

public static class PoolConnectionString
{
    // The old driver resolves a repeated key from either its first or its last occurrence,
    // depending on the setting, so the overlay precedes and follows the base string and applies
    // whichever occurrence is read.
    public static string Wrap(string baseConnectionString, string settings)
    {
        if (TestDriver.Current != TestDriver.Kind.Old)
            return baseConnectionString.Trim().TrimEnd(';') + ";" + settings;
        var application = UniqueApplication();
        var overlay = Overlay(settings, application);
        var basePart = baseConnectionString.Trim().TrimEnd(';');
        return overlay + basePart + ";" + overlay;
    }

    public static string OldDriverOnly(string settings) =>
        TestDriver.Current == TestDriver.Kind.Old ? settings : string.Empty;

    public static TimeSpan LowerRetryTimeoutFloorForOldDriver()
    {
        var property = DefaultRetryTimeoutProperty();
        var original = (TimeSpan)property.GetValue(null)!;
        property.SetValue(null, TimeSpan.FromSeconds(1));
        return original;
    }

    public static void RestoreRetryTimeoutFloorForOldDriver(TimeSpan original) =>
        DefaultRetryTimeoutProperty().SetValue(null, original);

    // The old driver raises a connection-string retryTimeout below its non-public
    // SFSessionHttpClientProperties.DefaultRetryTimeout (300 seconds today) to that
    // value, so only changing the property shortens retries. This throws when the type
    // or property is absent.
    private static PropertyInfo DefaultRetryTimeoutProperty()
    {
        var propertiesType = typeof(OldSnowflakeDbConnection).Assembly
            .GetType("Snowflake.Data.Core.SFSessionHttpClientProperties")
            ?? throw new InvalidOperationException(
                "The old driver has no SFSessionHttpClientProperties type to lower the retry timeout floor on.");
        return propertiesType.GetProperty(
                "DefaultRetryTimeout",
                BindingFlags.Static | BindingFlags.NonPublic | BindingFlags.Public)
            ?? throw new InvalidOperationException(
                "The old driver has no DefaultRetryTimeout property to lower the retry timeout floor.");
    }

    private static string Overlay(string settings, string application)
    {
        var parts = new List<string> { $"application={application}" };
        var trimmed = StripSetting(settings.Trim().TrimEnd(';'), "application");
        if (trimmed.Length > 0)
            parts.Add(trimmed);
        if (!HasSetting(settings, "waitingForIdleSessionTimeout"))
            parts.Add("waitingForIdleSessionTimeout=1s");
        if (!HasSetting(settings, "connection_timeout"))
            parts.Add("connection_timeout=10");
        if (!HasSetting(settings, "retryTimeout") && !HasSetting(settings, "retry_timeout"))
        {
            // Both spellings are set so the old driver applies the value whichever one it reads.
            parts.Add("retryTimeout=1");
            parts.Add("retry_timeout=1");
        }
        if (!HasSetting(settings, "disableRetry"))
            parts.Add("disableRetry=true");
        if (!HasSetting(settings, "maxHttpRetries"))
            parts.Add("maxHttpRetries=0");
        parts.Add($"application={application}");
        return string.Join(";", parts) + ";";
    }

    private static string UniqueApplication()
    {
        var hex = Guid.NewGuid().ToString("N");
        return "P" + hex.Substring(0, 8);
    }

    private static bool HasSetting(string settings, string key)
    {
        foreach (var part in settings.Split(new[] { ';' }, StringSplitOptions.RemoveEmptyEntries))
        {
            var eq = part.IndexOf('=');
            if (eq > 0 && part.Substring(0, eq).Trim().Equals(key, StringComparison.OrdinalIgnoreCase))
                return true;
        }
        return false;
    }

    private static string StripSetting(string settings, string key)
    {
        var kept = new List<string>();
        foreach (var part in settings.Split(new[] { ';' }, StringSplitOptions.RemoveEmptyEntries))
        {
            var trimmed = part.Trim();
            var eq = trimmed.IndexOf('=');
            if (eq <= 0)
                continue;
            var name = trimmed.Substring(0, eq);
            if (name.Equals(key, StringComparison.OrdinalIgnoreCase))
                continue;
            kept.Add(trimmed);
        }
        return string.Join(";", kept);
    }
}
