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
        var priorFailures = new List<JunitRetryXml.AttemptFailure>();

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
                RecordJunit(ctxt, test, delayedMessageBus, result, priorFailures);
                delayedMessageBus.Dispose();
                return result;
            }

            var failedAttempt = FailureFrom(delayedMessageBus);
            if (failedAttempt is { } failure)
                priorFailures.Add(failure);

            TestContext.Current.SendDiagnosticMessage(
                "Execution of '{0}' ended with a failure (attempt #{1}), retrying...",
                test.TestDisplayName, runCount);
            ctxt.Aggregator.Clear();
        }
    }

    private static void RecordJunit(
        SnowflakeCaseRunnerContext ctxt,
        IXunitTest test,
        SnowflakeDelayedMessageBus bus,
        RunSummary result,
        List<JunitRetryXml.AttemptFailure> priorFailures)
    {
        var classname = ctxt.TestCase.TestClassName ?? "unknown";
        var name = test.TestDisplayName;
        var time = result.Time;
        var skipped = bus.Messages.OfType<ITestSkipped>().Any();
        var failed = FailureFrom(bus);

        JunitRetryXml.CaseStatus status;
        if (skipped && result.Failed == 0)
            status = JunitRetryXml.CaseStatus.Skipped;
        else if (result.Failed != 0)
            status = JunitRetryXml.CaseStatus.Failed;
        else
            status = JunitRetryXml.CaseStatus.Passed;

        JunitRetryXml.Record(classname, name, time, status, priorFailures, failed);
    }

    private static JunitRetryXml.AttemptFailure? FailureFrom(SnowflakeDelayedMessageBus bus)
    {
        var failed = bus.Messages.OfType<ITestFailed>().LastOrDefault();
        if (failed is null)
            return null;
        var type = failed.ExceptionTypes?.FirstOrDefault() ?? "Exception";
        var message = failed.Messages is null ? "" : string.Join("\n", failed.Messages);
        var stack = failed.StackTraces is null ? null : string.Join("\n", failed.StackTraces);
        return new JunitRetryXml.AttemptFailure(type, message, stack);
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
