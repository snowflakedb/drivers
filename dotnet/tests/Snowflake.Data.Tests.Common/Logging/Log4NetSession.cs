using log4net;
using log4net.Appender;
using log4net.Core;
using log4net.Repository.Hierarchy;
using Microsoft.Extensions.Logging;
using ILogger = Microsoft.Extensions.Logging.ILogger;
using ILoggerFactory = Microsoft.Extensions.Logging.ILoggerFactory;
using LogLevel = Microsoft.Extensions.Logging.LogLevel;

namespace Snowflake.Data.Tests.Logging;

/// <summary>
/// A log4net repository private to one test, recording into a <see cref="MemoryAppender"/>,
/// with an <see cref="ILogger"/> that writes into it through the log4net provider.
/// </summary>
public sealed class Log4NetSession : IDisposable
{
    private readonly string _repositoryName;
    private readonly ILoggerFactory _factory;
    private readonly MemoryAppender _appender;

    private Log4NetSession(string repositoryName, ILoggerFactory factory, MemoryAppender appender, ILogger logger)
    {
        _repositoryName = repositoryName;
        _factory = factory;
        _appender = appender;
        Logger = logger;
    }

    public ILogger Logger { get; }

    public LoggingEvent[] Events => _appender.GetEvents();

    public static string MessageOf(LoggingEvent evt) =>
        evt.RenderedMessage ?? evt.MessageObject?.ToString() ?? string.Empty;

    public static Log4NetSession Start(Level threshold)
    {
        var repositoryName = Guid.NewGuid().ToString("N");
        var repository = LogManager.CreateRepository(repositoryName);
        var appender = new MemoryAppender { Name = "Memory" };
        appender.ActivateOptions();
        var hierarchy = (Hierarchy)repository;
        hierarchy.Root.Level = threshold;
        hierarchy.Root.AddAppender(appender);
        hierarchy.Configured = true;

        var factory = new Microsoft.Extensions.Logging.LoggerFactory(
            new ILoggerProvider[]
            {
                new Log4NetProvider(new Log4NetProviderOptions
                {
                    LoggerRepository = repositoryName,
                    ExternalConfigurationSetup = true,
                }),
            },
            new LoggerFilterOptions { MinLevel = LogLevel.Trace });

        return new Log4NetSession(repositoryName, factory, appender, factory.CreateLogger("Snowflake.Data"));
    }

    public void Dispose()
    {
        _factory.Dispose();
        LogManager.ShutdownRepository(_repositoryName);
    }
}
