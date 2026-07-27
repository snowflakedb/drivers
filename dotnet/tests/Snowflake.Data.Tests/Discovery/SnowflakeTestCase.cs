using System.ComponentModel;
using Xunit.Sdk;

#if !OLD_XUNIT

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

#else
namespace Snowflake.Data.Tests.Discovery;

public sealed class SnowflakeTestCase : LongLivedMarshalByRefObject, IXunitTestCase
{
    private readonly IXunitTestCase _inner;
    private readonly int _maxRetriesCount;

    [Obsolete("Used for deserialization")]
    public SnowflakeTestCase()
    {
        _inner = null!;
    }

    public SnowflakeTestCase(IXunitTestCase inner, int maxRetriesCount)
    {
        _inner = inner;
        _maxRetriesCount = maxRetriesCount;
    }

    public void Deserialize(IXunitSerializationInfo info) => _inner.Deserialize(info);
    public void Serialize(IXunitSerializationInfo info) => _inner.Serialize(info);

    public string DisplayName => _inner.DisplayName;
    public string SkipReason => _inner.SkipReason;

    public ISourceInformation SourceInformation
    {
        get => _inner.SourceInformation;
        set => _inner.SourceInformation = value;
    }

    public ITestMethod TestMethod => _inner.TestMethod;
    public object[] TestMethodArguments => _inner.TestMethodArguments;
    public Dictionary<string, List<string>> Traits => _inner.Traits;
    public string UniqueID => _inner.UniqueID;
    public Exception InitializationException => _inner.InitializationException;
    public IMethodInfo Method => _inner.Method;
    public int Timeout => _inner.Timeout;

    public async Task<RunSummary> RunAsync(
        IMessageSink diagnosticMessageSink,
        IMessageBus messageBus,
        object[] constructorArguments,
        ExceptionAggregator aggregator,
        CancellationTokenSource cancellationTokenSource)
    {
        var messageBusDecorator = new SnowflakeMessageBus(messageBus, _maxRetriesCount);

        var retriesCount = 0;
        RunSummary baseResult;
        do
        {
            if (retriesCount > 1)
                await Task.Delay(500, cancellationTokenSource.Token).ConfigureAwait(false);

            baseResult = await _inner
                .RunAsync(diagnosticMessageSink, messageBusDecorator, constructorArguments, aggregator, cancellationTokenSource)
                .ConfigureAwait(false);
        } while (retriesCount++ < _maxRetriesCount && baseResult.Failed > 0);

        baseResult.Failed -= messageBusDecorator.SkippedCount;
        baseResult.Skipped += messageBusDecorator.SkippedCount;
        return baseResult;
    }
}
#endif
