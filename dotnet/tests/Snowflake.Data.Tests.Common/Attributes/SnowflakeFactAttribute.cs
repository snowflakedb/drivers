using System.Runtime.CompilerServices;
using Snowflake.Data.Tests.Discovery;

namespace Snowflake.Data.Tests.Attributes;

[XunitTestCaseDiscoverer(typeof(SnowflakeTestCaseDiscoverer))]
public sealed class SnowflakeFactAttribute : FactAttribute
{
    public RetriesCount RetriesCount { get; set; }

    public SnowflakeFactAttribute(
        SkipCondition skip = SkipCondition.None,
        string skipMessage = "",
        RetriesCount retriesCount = RetriesCount.Twice,
        [CallerFilePath] string? sourceFilePath = null,
        [CallerLineNumber] int sourceLineNumber = -1)
        : base(sourceFilePath!, sourceLineNumber)
    {
        RetriesCount = retriesCount;
        var skipEvaluationResult = SkipConditionEvaluator.Evaluate(skip);

        if (skipEvaluationResult.ShouldSkip)
            Skip = skipEvaluationResult.SkipMessage + skipMessage;
    }
}

public enum RetriesCount
{
    NoRetries = 0,
    Once = 1,
    Twice = 2,
    Thrice = 3,
}
