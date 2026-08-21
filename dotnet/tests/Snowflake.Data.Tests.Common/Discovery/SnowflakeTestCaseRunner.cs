using Xunit.Sdk;

namespace Snowflake.Data.Tests.Discovery;

public class SnowflakeTestCaseRunner : XunitTestCaseRunnerBase<SnowflakeCaseRunnerContext, IXunitTestCase, IXunitTest>
{
    public static SnowflakeTestCaseRunner Instance { get; } = new();

    public async ValueTask<RunSummary> Run(
        int maxRetries,
        IXunitTestCase testCase,
        IMessageBus messageBus,
        ExceptionAggregator aggregator,
        CancellationTokenSource cancellationTokenSource,
        string displayName,
        string? skipReason,
        ExplicitOption explicitOption,
        object?[] constructorArguments,
        FixtureMappingManager fixtureMappingManager)
    {
        var tests = await aggregator.RunAsync(testCase.CreateTests, []).ConfigureAwait(false);

        if (aggregator.ToException() is { } ex)
        {
            if (ex.Message.StartsWith(DynamicSkipToken.Value, StringComparison.Ordinal))
                return XunitRunnerHelper.SkipTestCases(
                    messageBus,
                    cancellationTokenSource,
                    [testCase],
                    ex.Message.Substring(DynamicSkipToken.Value.Length),
                    sendTestCaseMessages: false
                );

            return XunitRunnerHelper.FailTestCases(
                messageBus,
                cancellationTokenSource,
                [testCase],
                ex,
                sendTestCaseMessages: false
            );
        }

        await using var ctxt = new SnowflakeCaseRunnerContext(maxRetries, testCase, tests, messageBus, aggregator, cancellationTokenSource,
            displayName, skipReason, explicitOption, constructorArguments, fixtureMappingManager);
        await ctxt.InitializeAsync().ConfigureAwait(false);

        return await Run(ctxt).ConfigureAwait(false);
    }

    protected override async ValueTask<RunSummary> RunTest(
        SnowflakeCaseRunnerContext ctxt,
        IXunitTest test)
    {
        var runCount = 0;
        var maxRetries = ctxt.MaxRetries;

        if (maxRetries < 0)
            maxRetries = 3;

        for (; ; )
        {
            var backoffDelay = 500 * ((1 << runCount) - 1);
            await Task.Delay(backoffDelay).ConfigureAwait(false);

            var delayedMessageBus = new SnowflakeDelayedMessageBus(ctxt.MessageBus);
            var aggregator = ctxt.Aggregator.Clone();
            var result = await XunitTestRunner.Instance.Run(
                test,
                delayedMessageBus,
                ctxt.ConstructorArguments,
                ctxt.ExplicitOption,
                aggregator,
                ctxt.CancellationTokenSource,
                ctxt.BeforeAfterTestAttributes,
                ctxt.FixtureMappingManager
            ).ConfigureAwait(false);

            if (!(aggregator.HasExceptions || result.Failed != 0) || ++runCount > maxRetries)
            {
                delayedMessageBus.Dispose();
                return result;
            }

            TestContext.Current.SendDiagnosticMessage(
                "Execution of '{0}' ended with a failure (attempt #{1}), retrying...",
                test.TestDisplayName, runCount);
            ctxt.Aggregator.Clear();
        }
    }
}

public class SnowflakeCaseRunnerContext(
    int maxRetries,
    IXunitTestCase testCase,
    IReadOnlyCollection<IXunitTest> tests,
    IMessageBus messageBus,
    ExceptionAggregator aggregator,
    CancellationTokenSource cancellationTokenSource,
    string displayName,
    string? skipReason,
    ExplicitOption explicitOption,
    object?[] constructorArguments,
    FixtureMappingManager fixtureMappingManager) :
    XunitTestCaseRunnerBaseContext<IXunitTestCase, IXunitTest>(testCase, tests, messageBus, aggregator, cancellationTokenSource, displayName,
        skipReason, explicitOption, constructorArguments, fixtureMappingManager)
{
    public int MaxRetries { get; } = maxRetries;
    public FixtureMappingManager FixtureMappingManager { get; } = fixtureMappingManager;
}
