using NLog;
using NLog.Config;
using NLog.Extensions.Logging;
using NLog.Targets;
using ILogger = Microsoft.Extensions.Logging.ILogger;
using ILoggerFactory = Microsoft.Extensions.Logging.ILoggerFactory;
using LogLevel = Microsoft.Extensions.Logging.LogLevel;
using NLogLevel = NLog.LogLevel;

namespace Snowflake.Data.Tests.Logging;

public sealed class NLogSession : ICapturedLogSession
{
    private readonly LogFactory _nlogFactory;
    private readonly ILoggerFactory _factory;
    private readonly MemoryTarget _target;

    private NLogSession(LogFactory nlogFactory, ILoggerFactory factory, MemoryTarget target, ILogger logger)
    {
        _nlogFactory = nlogFactory;
        _factory = factory;
        _target = target;
        Logger = logger;
    }

    public ILogger Logger { get; }

    public IReadOnlyList<CapturedLogEvent> GetEvents() =>
        _target.Logs.Select(Parse).ToArray();

    public static NLogSession Start(LogLevel minimum)
    {
        var nlogFactory = new LogFactory();
        var target = new MemoryTarget("memory") { Layout = "${level:uppercase=true}|${message}" };
        var config = new LoggingConfiguration(nlogFactory);
        config.AddRule(ToNLogLevel(minimum), NLogLevel.Off, target);
        nlogFactory.Configuration = config;
        var factory = new NLogLoggerFactory(new NLogLoggerProvider(new NLogProviderOptions(), nlogFactory));
        return new NLogSession(nlogFactory, factory, target, factory.CreateLogger("Snowflake.Data"));
    }

    public void Dispose()
    {
        _factory.Dispose();
        _nlogFactory.Dispose();
    }

    private static CapturedLogEvent Parse(string line)
    {
        var separator = line.IndexOf('|');
        var levelName = separator < 0 ? string.Empty : line.Substring(0, separator);
        var message = separator < 0 ? line : line.Substring(separator + 1);
        return new CapturedLogEvent(ToLogLevel(levelName), message);
    }

    private static NLogLevel ToNLogLevel(LogLevel minimum) => minimum switch
    {
        LogLevel.Critical => NLogLevel.Fatal,
        LogLevel.Error => NLogLevel.Error,
        LogLevel.Warning => NLogLevel.Warn,
        LogLevel.Information => NLogLevel.Info,
        LogLevel.Debug => NLogLevel.Debug,
        _ => NLogLevel.Trace,
    };

    private static LogLevel ToLogLevel(string levelName) => levelName switch
    {
        "FATAL" => LogLevel.Critical,
        "ERROR" => LogLevel.Error,
        "WARN" => LogLevel.Warning,
        "INFO" => LogLevel.Information,
        "DEBUG" => LogLevel.Debug,
        "TRACE" => LogLevel.Trace,
        _ => LogLevel.None,
    };
}

public sealed class NLogSessionFactory : ILogSessionFactory
{
    public ICapturedLogSession Start(LogLevel minimum) => NLogSession.Start(minimum);
}
