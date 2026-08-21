using Xunit.Sdk;

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
