using System.Diagnostics;
using Microsoft.Extensions.Logging;

namespace Snowflake.Data.Core;

// TODO this is temp component, will be removed in SNOW-4382573 + SNOW-4382577
internal static class LoggerAndTelemetryStubs
{
    internal static ILogger<T> GetLogger<T>() => new Logger<T>();

    internal static Activity? StartActivity(string name) => new(name);

    internal sealed class Logger<T> : ILogger<T>
    {
        public void Log<TState>(LogLevel logLevel, EventId eventId, TState state, Exception? exception, Func<TState, Exception?, string> formatter) =>
            Console.WriteLine($"[{logLevel}]  {formatter(state, exception)}");

        public bool IsEnabled(LogLevel logLevel) => true;

        public IDisposable? BeginScope<TState>(TState state) where TState : notnull =>
            throw new NotImplementedException("TODO this is temp component, will be removed in SNOW-4382573");
    }
}
