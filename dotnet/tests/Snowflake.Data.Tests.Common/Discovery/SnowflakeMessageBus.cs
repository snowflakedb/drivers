using Xunit.Sdk;

#if !OLD_XUNIT

namespace Snowflake.Data.Tests.Discovery;

public sealed class SnowflakeDelayedMessageBus(IMessageBus innerBus) : IMessageBus
{
    private readonly List<IMessageSinkMessage> _messages = [];
    private static readonly TestPerformanceRecorder PerformanceRecorder = new();

    public bool QueueMessage(IMessageSinkMessage message)
    {
        _messages.Add(message);
        return true;
    }

    public void Dispose()
    {
        foreach (var message in _messages)
        {
            innerBus.QueueMessage(message);

            if (message is ITestResultMessage testResultMessage)
                PerformanceRecorder.AddEntry(testResultMessage);
        }
    }
}

#else

namespace Snowflake.Data.Tests.Discovery;

public class SnowflakeMessageBus : IMessageBus
{
    private const string XunitSdkSkipException = "Xunit.Sdk.SkipException";
    private readonly IMessageBus _messageBusImplementation;
    private int _retriesCountRemaining;
    private readonly Queue<IMessageSinkMessage> _messages = new();
    private bool _isMessageProcessingDelayed;

    public int SkippedCount { get; private set; }

    private static readonly TestPerformanceRecorder PerformanceRecorder = new();

    public SnowflakeMessageBus(IMessageBus messageBusImplementation, int retriesCount)
    {
        _messageBusImplementation = messageBusImplementation;
        _retriesCountRemaining = retriesCount;
        _isMessageProcessingDelayed = retriesCount > 0;
    }

    public void Dispose() => _messageBusImplementation.Dispose();

    public bool QueueMessage(IMessageSinkMessage message)
    {
        if (message is TestCaseStarting)
            _messages.Clear();

        if (message is ITestPassed or ITestSkipped)
        {
            foreach (var delayedMessage in _messages)
            {
                _isMessageProcessingDelayed = false;
                _messageBusImplementation.QueueMessage(delayedMessage);
            }

            PerformanceRecorder.AddEntry((ITestResultMessage)message);
        }

        if (message is not ITestFailed testFailed)
            return DelayQueueMessage(message);

        var anySkipped = testFailed.ExceptionTypes
            .Select((x, i) => (Type: x, Index: i))
            .FirstOrDefault(x => XunitSdkSkipException.Equals(x.Type));

        if (anySkipped != default)
        {
            SkippedCount++;
            var skipReason = testFailed.Messages[anySkipped.Index];
            var skippedMessage = new TestSkipped(testFailed.Test, skipReason);

            while (_messages.Count > 0)
            {
                var delayedMessage = _messages.Dequeue();
                _messageBusImplementation.QueueMessage(delayedMessage);
            }

            return _messageBusImplementation.QueueMessage(skippedMessage);
        }

        var result = DelayQueueMessage(message);

        if (_retriesCountRemaining == 0)
            PerformanceRecorder.AddEntry(testFailed);

        if (_retriesCountRemaining-- <= 0)
            _isMessageProcessingDelayed = false;

        return result;
    }

    private bool DelayQueueMessage(IMessageSinkMessage message)
    {
        if (_isMessageProcessingDelayed)
        {
            _messages.Enqueue(message);
            return true;
        }

        return _messageBusImplementation.QueueMessage(message);
    }
}
#endif
