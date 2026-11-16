using log4net;
using log4net.Appender;
using log4net.Core;
using log4net.Repository.Hierarchy;
using Microsoft.Extensions.Logging;
using ILogger = Microsoft.Extensions.Logging.ILogger;
using ILoggerFactory = Microsoft.Extensions.Logging.ILoggerFactory;
using LogLevel = Microsoft.Extensions.Logging.LogLevel;

namespace Snowflake.Data.Tests.Logging;

public sealed class Log4NetSession : ICapturedLogSession
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

    public IReadOnlyList<CapturedLogEvent> GetEvents() =>
        _appender.GetEvents().Select(evt => new CapturedLogEvent(ToLogLevel(evt.Level), MessageOf(evt))).ToArray();

    public static Log4NetSession Start(LogLevel minimum)
    {
        var repositoryName = Guid.NewGuid().ToString("N");
        var repository = LogManager.CreateRepository(repositoryName);
        var appender = new MemoryAppender { Name = "Memory" };
        appender.ActivateOptions();
        var hierarchy = (Hierarchy)repository;
        hierarchy.Root.Level = ToLog4NetLevel(minimum);
        hierarchy.Root.AddAppender(appender);
        hierarchy.Configured = true;

        var provider = new Log4NetProvider(new Log4NetProviderOptions
        {
            LoggerRepository = repositoryName,
            ExternalConfigurationSetup = true,
        });
        var factory = new LoggerFactory([provider], new LoggerFilterOptions { MinLevel = LogLevel.Trace });

        return new Log4NetSession(repositoryName, factory, appender, factory.CreateLogger("Snowflake.Data"));
    }

    public void Dispose()
    {
        _factory.Dispose();
        LogManager.ShutdownRepository(_repositoryName);
    }

    private static string MessageOf(LoggingEvent evt) =>
        evt.RenderedMessage ?? evt.MessageObject?.ToString() ?? string.Empty;

    private static Level ToLog4NetLevel(LogLevel minimum) => minimum switch
    {
        LogLevel.Critical => Level.Fatal,
        LogLevel.Error => Level.Error,
        LogLevel.Warning => Level.Warn,
        LogLevel.Information => Level.Info,
        LogLevel.Debug => Level.Debug,
        _ => Level.All,
    };

    private static LogLevel ToLogLevel(Level? level)
    {
        if (level is null)
            return LogLevel.None;
        if (level == Level.Fatal)
            return LogLevel.Critical;
        if (level == Level.Error)
            return LogLevel.Error;
        if (level == Level.Warn)
            return LogLevel.Warning;
        if (level == Level.Info)
            return LogLevel.Information;
        if (level == Level.Debug)
            return LogLevel.Debug;
        if (level == Level.Trace)
            return LogLevel.Trace;
        return LogLevel.None;
    }
}

public sealed class Log4NetSessionFactory : ILogSessionFactory
{
    public ICapturedLogSession Start(LogLevel minimum) => Log4NetSession.Start(minimum);
}
