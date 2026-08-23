using System.Runtime.CompilerServices;

#if !OLD_XUNIT
using Snowflake.Data.Tests.Discovery;
#else
#endif

namespace Snowflake.Data.Tests.Attributes;

#if !OLD_XUNIT
[XunitTestCaseDiscoverer(typeof(SnowflakeTheoryDiscovererV3))]
#else
[XunitTestCaseDiscoverer("Snowflake.Data.Tests.Discovery.SnowflakeTheoryDiscoverer", "Snowflake.Data.Tests.Common")]
#endif
public sealed class SnowflakeTheoryAttribute : TheoryAttribute
{
    public RetriesCount RetriesCount { get; set; }

    public SnowflakeTheoryAttribute(
        SkipCondition skip = SkipCondition.None,
        RetriesCount retriesCount = RetriesCount.NoRetries,
        [CallerFilePath] string? sourceFilePath = null,
        [CallerLineNumber] int sourceLineNumber = -1)
#if !OLD_XUNIT
        : base(sourceFilePath!, sourceLineNumber)
#endif
    {
        RetriesCount = retriesCount;
        var skipEvaluationResult = SkipConditionEvaluator.Evaluate(skip);

        if (skipEvaluationResult.ShouldSkip)
            Skip = skipEvaluationResult.SkipMessage;
    }
}
