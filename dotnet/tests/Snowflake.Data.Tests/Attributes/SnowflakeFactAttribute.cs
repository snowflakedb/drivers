using System.Runtime.CompilerServices;

#if !OLD_XUNIT
using Xunit.v3;
using Snowflake.Data.Tests.Discovery;
#else
using Xunit.Sdk;
#endif

namespace Snowflake.Data.Tests.Attributes;

#if !OLD_XUNIT
[XunitTestCaseDiscoverer(typeof(SnowflakeTestCaseDiscovererV3))]
#else
[XunitTestCaseDiscoverer("Snowflake.Data.Tests.Discovery.SnowflakeTestCaseDiscoverer", "Snowflake.Data.Tests")]
#endif
public sealed class SnowflakeFactAttribute : FactAttribute
{
    public RetriesCount RetriesCount { get; set; }

    public SnowflakeFactAttribute(
        SkipCondition skip = SkipCondition.None,
        RetriesCount retriesCount = RetriesCount.Once,
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

public enum RetriesCount
{
    NoRetries = 0,
    Once = 1,
    Twice = 2,
    Thrice = 3,
}
