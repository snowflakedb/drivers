using Xunit.Sdk;

namespace Snowflake.Data.Tests.Discovery;

public sealed class SnowflakeDelayedMessageBus : IMessageBus
{
    private readonly List<IMessageSinkMessage> _messages = [];
    private static readonly TestPerformanceRecorder PerformanceRecorder = new();
    private readonly IMessageBus _innerBus;

    public SnowflakeDelayedMessageBus(IMessageBus innerBus)
    {
        _innerBus = innerBus;
    }

    internal IReadOnlyList<IMessageSinkMessage> Messages => _messages;

    public bool QueueMessage(IMessageSinkMessage message)
    {
        _messages.Add(message);
        return true;
    }

    public void Dispose()
    {
        foreach (var message in _messages)
        {
            _innerBus.QueueMessage(message);

            if (message is ITestResultMessage testResultMessage)
                PerformanceRecorder.AddEntry(testResultMessage);
        }
    }
}
