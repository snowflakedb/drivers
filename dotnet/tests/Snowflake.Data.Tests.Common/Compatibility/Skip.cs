using Xunit.Sdk;

namespace Snowflake.Data.Tests.Compatibility;

public static class Skip
{
    public static void FutureMilestone(string rationale = "TODO not yet implemented") =>
        When(TestDriver.Current == TestDriver.Kind.New, rationale);

    // TODO(SNOW-4382580): remove once the new driver honours DOTNET_QUERY_RESULT_FORMAT.
    public static void UntilNewDriverSelectsResultFormat(string resultFormat) =>
        When(
            TestDriver.Current == TestDriver.Kind.New && string.Equals(resultFormat, "JSON", StringComparison.OrdinalIgnoreCase),
            "the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so a JSON row would read an Arrow result");

    public static void FutureMilestone(SkipCondition condition, string rationale)
    {
        if (TestDriver.Current == TestDriver.Kind.New)
            For(condition, rationale);
    }

    public static void When(bool condition, string rationale)
    {
        if (condition)
            throw SkipException.ForSkip(rationale);
    }

    public static void For(SkipCondition condition, string rationale)
    {
        var conditionResult = SkipConditionEvaluator.Evaluate(condition);

        if (conditionResult.ShouldSkip)
            throw SkipException.ForSkip(rationale);
    }

    public static void OnOldDriver(string differenceId, string rationale) =>
        When(TestDriver.Current == TestDriver.Kind.Old, $"{differenceId}: {rationale}");

    public static void OnNewDriver(string differenceId, string rationale) =>
        When(TestDriver.Current == TestDriver.Kind.New, $"{differenceId}: {rationale}");

    public static void ForJsonResultSet(string rationale) =>
        When(string.Equals(Environment.GetEnvironmentVariable("QUERY_RESULT_FORMAT"), "JSON", StringComparison.OrdinalIgnoreCase), rationale);
}
