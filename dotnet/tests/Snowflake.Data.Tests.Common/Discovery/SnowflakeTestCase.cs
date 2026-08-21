using System.ComponentModel;
using Xunit.Sdk;

namespace Snowflake.Data.Tests.Discovery;

public class SnowflakeTestCase : XunitTestCase, ISelfExecutingXunitTestCase
{
    [EditorBrowsable(EditorBrowsableState.Never)]
    [Obsolete("Called by the de-serializer; should only be called by deriving classes for de-serialization purposes")]
    public SnowflakeTestCase() { }

    public SnowflakeTestCase(
        int maxRetries,
        IXunitTestMethod testMethod,
        string testCaseDisplayName,
        string uniqueID,
        bool @explicit,
        Type[]? skipExceptions = null,
        string? skipReason = null,
        Type? skipType = null,
        string? skipUnless = null,
        string? skipWhen = null,
        Dictionary<string, HashSet<string>>? traits = null,
        object?[]? testMethodArguments = null,
        string? sourceFilePath = null,
        int? sourceLineNumber = null,
        int? timeout = null) :
            base(testMethod, testCaseDisplayName, uniqueID, @explicit, skipExceptions, skipReason, skipType, skipUnless, skipWhen, traits, testMethodArguments, sourceFilePath, sourceLineNumber, timeout)
    {
        MaxRetries = maxRetries;
    }

    public int MaxRetries { get; private set; }

    protected override void Deserialize(IXunitSerializationInfo info)
    {
        base.Deserialize(info);
        MaxRetries = info.GetValue<int>(nameof(MaxRetries));
    }

    public ValueTask<RunSummary> Run(
        ExplicitOption explicitOption,
        IMessageBus messageBus,
        object?[] constructorArguments,
        FixtureMappingManager fixtureMappingManager,
        ExceptionAggregator aggregator,
        CancellationTokenSource cancellationTokenSource) =>
            SnowflakeTestCaseRunner.Instance.Run(
                MaxRetries,
                this,
                messageBus,
                aggregator.Clone(),
                cancellationTokenSource,
                TestCaseDisplayName,
                SkipReason,
                explicitOption,
                constructorArguments,
                fixtureMappingManager
            );

    protected override void Serialize(IXunitSerializationInfo info)
    {
        base.Serialize(info);
        info.AddValue(nameof(MaxRetries), MaxRetries);
    }
}

public class SnowflakeEnumeratedTestCase : XunitDelayEnumeratedTheoryTestCase, ISelfExecutingXunitTestCase
{
    [Obsolete("Called by the de-serializer; should only be called by deriving classes for de-serialization purposes")]
    public SnowflakeEnumeratedTestCase() { }

    public SnowflakeEnumeratedTestCase(
        int maxRetries,
        IXunitTestMethod testMethod,
        string testCaseDisplayName,
        string uniqueID,
        bool @explicit,
        bool skipTestWithoutData,
        Type[]? skipExceptions = null,
        string? skipReason = null,
        Type? skipType = null,
        string? skipUnless = null,
        string? skipWhen = null,
        Dictionary<string, HashSet<string>>? traits = null,
        string? sourceFilePath = null,
        int? sourceLineNumber = null,
        int? timeout = null) :
            base(testMethod, testCaseDisplayName, uniqueID, @explicit, skipTestWithoutData, skipExceptions, skipReason, skipType, skipUnless, skipWhen, traits, sourceFilePath, sourceLineNumber, timeout)
    {
        MaxRetries = maxRetries;
    }

    public int MaxRetries { get; private set; }

    protected override void Deserialize(IXunitSerializationInfo info)
    {
        base.Deserialize(info);
        MaxRetries = info.GetValue<int>(nameof(MaxRetries));
    }

    public ValueTask<RunSummary> Run(
        ExplicitOption explicitOption,
        IMessageBus messageBus,
        object?[] constructorArguments,
        FixtureMappingManager fixtureMappingManager,
        ExceptionAggregator aggregator,
        CancellationTokenSource cancellationTokenSource) =>
            SnowflakeTestCaseRunner.Instance.Run(
                MaxRetries,
                this,
                messageBus,
                aggregator.Clone(),
                cancellationTokenSource,
                TestCaseDisplayName,
                SkipReason,
                explicitOption,
                constructorArguments,
                fixtureMappingManager
            );

    protected override void Serialize(IXunitSerializationInfo info)
    {
        base.Serialize(info);
        info.AddValue(nameof(MaxRetries), MaxRetries);
    }
}
