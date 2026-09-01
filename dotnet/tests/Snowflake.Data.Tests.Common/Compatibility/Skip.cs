using Xunit.Sdk;

namespace Snowflake.Data.Tests.Compatibility;

public static class Skip
{
    public static void FutureMilestone() => When(true, "TODO not yet implemented");

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
}
