using System.Runtime.CompilerServices;
using Snowflake.Data.Tests.Discovery;

namespace Snowflake.Data.Tests.Attributes;

[XunitTestCaseDiscoverer(typeof(SnowflakeTheoryDiscoverer))]
public sealed class SnowflakeTheoryAttribute : TheoryAttribute
{
    public RetriesCount RetriesCount { get; set; }

    public SnowflakeTheoryAttribute(
        SkipCondition skip = SkipCondition.None,
        RetriesCount retriesCount = RetriesCount.NoRetries,
        [CallerFilePath] string? sourceFilePath = null,
        [CallerLineNumber] int sourceLineNumber = -1)
        : base(sourceFilePath!, sourceLineNumber)
    {
        RetriesCount = retriesCount;
        var skipEvaluationResult = SkipConditionEvaluator.Evaluate(skip);

        if (skipEvaluationResult.ShouldSkip)
            Skip = skipEvaluationResult.SkipMessage;
    }
}
