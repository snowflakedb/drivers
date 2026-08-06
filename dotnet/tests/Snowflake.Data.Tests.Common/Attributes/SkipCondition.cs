using System.Runtime.InteropServices;

namespace Snowflake.Data.Tests.Attributes;

[Flags]
public enum SkipCondition
{
    None = 0,

    // Platform (primitive bits)
    SkipOnWindows = 1 << 0,
    SkipOnLinux = 1 << 1,
    SkipOnMacOS = 1 << 2,

    // Platform composites
    RunOnlyOnWindows = SkipOnLinux | SkipOnMacOS,
    RunOnlyOnLinux = SkipOnWindows | SkipOnMacOS,
    RunOnlyOnMacOS = SkipOnWindows | SkipOnLinux,

    // CI
    SkipOnCI = 1 << 3,
    RunOnlyOnCI = 1 << 4,

    // Cloud provider (reads snowflake_cloud_env)
    SkipOnCloudAWS = 1 << 5,
    SkipOnCloudAzure = 1 << 6,
    SkipOnCloudGCP = 1 << 7,
    RunOnlyOnCloudAWS = SkipOnCloudAzure | SkipOnCloudGCP,
    RunOnlyOnCloudAzure = SkipOnCloudAWS | SkipOnCloudGCP,
    RunOnlyOnCloudGCP = SkipOnCloudAWS | SkipOnCloudAzure,
}

internal static class SkipConditionEvaluator
{
    internal readonly struct SkipEvaluationResult
    {
        internal readonly string? SkipMessage;
        internal readonly bool ShouldSkip;

        public SkipEvaluationResult(string? skipMessage, bool shouldSkip)
        {
            SkipMessage = skipMessage;
            ShouldSkip = shouldSkip;
        }
    }

    internal static SkipEvaluationResult Evaluate(SkipCondition condition)
    {
        var skipMessage = EvaluateInner(condition);
        return new(skipMessage, !string.IsNullOrEmpty(skipMessage));
    }

    private static string? EvaluateInner(SkipCondition condition)
    {
        if (condition == SkipCondition.None)
            return null;

        // Platform checks
        if (condition.HasFlag(SkipCondition.SkipOnWindows) && RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            return "Test is skipped on Windows.";

        if (condition.HasFlag(SkipCondition.SkipOnLinux) && RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
            return "Test is skipped on Linux.";

        if (condition.HasFlag(SkipCondition.SkipOnMacOS) && RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
            return "Test is skipped on macOS.";

        // CI checks
        if (condition.HasFlag(SkipCondition.SkipOnCI) && Environment.GetEnvironmentVariable("CI") == "true")
            return "Test is skipped on CI.";

        if (condition.HasFlag(SkipCondition.RunOnlyOnCI) && string.IsNullOrEmpty(Environment.GetEnvironmentVariable("CI")))
            return "Test runs only on CI.";

        // Cloud provider checks
        var cloudEnv = Environment.GetEnvironmentVariable("snowflake_cloud_env");

        if (condition.HasFlag(SkipCondition.SkipOnCloudAWS) && cloudEnv == "AWS")
            return "Test is skipped on AWS.";

        if (condition.HasFlag(SkipCondition.SkipOnCloudAzure) && cloudEnv == "AZURE")
            return "Test is skipped on Azure.";

        if (condition.HasFlag(SkipCondition.SkipOnCloudGCP) && cloudEnv == "GCP")
            return "Test is skipped on GCP.";

        return null;
    }
}
