extern alias OldDriver;
using System.Data.Common;
using System.Globalization;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E.Fixtures;
using OldSnowflakeDbConnection = OldDriver::Snowflake.Data.Client.SnowflakeDbConnection;
using OldSnowflakeDbException = OldDriver::Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.E2E;

[CollectionDefinition(nameof(EasyLoggingTestCollection), DisableParallelization = true)]
public sealed class EasyLoggingTestCollection : ICollectionFixture<EasyLoggingTestCollection.Fixture>
{
    public sealed class Fixture : IDisposable
    {
        public readonly string WorkingDirectory = Path.Combine(
            Path.GetTempPath(),
            $"easy_logging_test_configs_{Path.GetRandomFileName()}");

        public Fixture()
        {
            Directory.CreateDirectory(WorkingDirectory);
        }

        public void Dispose()
        {
            if (Directory.Exists(WorkingDirectory))
                Directory.Delete(WorkingDirectory, true);
        }
    }
}

[Collection(nameof(EasyLoggingTestCollection))]
public sealed class EasyLoggingTest : LiveE2ETestBase, IDisposable
{
    private const string LogDirectoryName = "dotnet";
    private const int InvalidConnectionStringErrorCode = 270008;
    private const string ConnectionExceptionSqlState = "08006";
    private const string EasyLoggingNotSupportedRationale = "the new driver does not read CLIENT_CONFIG_FILE or configure Easy Logging";
    private readonly EasyLoggingTestCollection.Fixture _classFixture;
    private readonly object? _appendersPriorToTest;
    private readonly EasyLoggingOldDriver.StarterState _starterStatePriorToTest;

    public EasyLoggingTest(
        E2EITFixture fixture,
        EasyLoggingTestCollection.Fixture classFixture,
        ITestOutputHelper output)
        : base(fixture, output)
    {
        _classFixture = classFixture;
        _starterStatePriorToTest = EasyLoggingOldDriver.CaptureStarterState();
        EasyLoggingOldDriver.ClearInitTrial();
        _appendersPriorToTest = EasyLoggingOldDriver.CaptureAppenders();
        EasyLoggingOldDriver.ReplaceAppenders(EasyLoggingOldDriver.CreateEmptyAppenderList());
    }

    public void Dispose()
    {
        EasyLoggingOldDriver.RestoreStarterState(_starterStatePriorToTest);
        EasyLoggingOldDriver.ReplaceAppenders(_appendersPriorToTest);

        var logDirectory = Path.Combine(_classFixture.WorkingDirectory, LogDirectoryName);
        if (Directory.Exists(logDirectory) && !RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            var result = Chmod(logDirectory, UnixMode.OwnerRwx);
            if (result != 0)
                throw new InvalidOperationException($"Failed to restore directory permissions in teardown for {logDirectory}");
        }
    }

    [SnowflakeFact]
    public async Task ShouldEnableEasyLoggingAsync()
    {
        Skip.OnNewDriver("BD#easy-logging-not-supported", EasyLoggingNotSupportedRationale);

        var configFilePath = CreateConfigTempFile(_classFixture.WorkingDirectory, Config("WARN", _classFixture.WorkingDirectory));
        using var conn = OpenConnectionWithConfig(configFilePath);
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.True(EasyLoggingOldDriver.HasEasyLoggingAppender());
    }

    [SnowflakeFact]
    public async Task ShouldFailToEnableEasyLoggingForWrongConfigurationAsync()
    {
        Skip.When(RuntimeInformation.IsOSPlatform(OSPlatform.Windows), "Test is skipped on Windows.");
        Skip.OnNewDriver("BD#easy-logging-not-supported", EasyLoggingNotSupportedRationale);

        var configFilePath = CreateConfigTempFile(_classFixture.WorkingDirectory, "random config content");
        using var conn = OpenConnectionWithConfig(configFilePath);
        var thrown = await AssertOpenThrowsSnowflakeDbExceptionAsync(conn, TestContext.Current.CancellationToken).ConfigureAwait(false);

        var messages = new[] { thrown.Message, thrown.InnerException?.Message };
        var concatenatedMessages = string.Join(Environment.NewLine, messages);
        Assert.Contains("Connection string is invalid: Unable to initialize session", concatenatedMessages);
        Assert.Empty(EasyLoggingOldDriver.GetAppenders());
    }

    [SnowflakeFact]
    public async Task ShouldReCreateEasyLoggingUnixLogFileWithCustomisedPermissionsAsync()
    {
        Skip.When(RuntimeInformation.IsOSPlatform(OSPlatform.Windows), "Test is skipped on Windows.");
        Skip.OnNewDriver("BD#easy-logging-not-supported", EasyLoggingNotSupportedRationale);

        var configFilePath = CreateConfigTempFile(
            _classFixture.WorkingDirectory,
            Config("WARN", _classFixture.WorkingDirectory, "640"));
        using var conn = OpenConnectionWithConfig(configFilePath);
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        var logger = EasyLoggingOldDriver.GetLogger(typeof(EasyLoggingTest));
        var logFile = EasyLoggingOldDriver.FirstAppenderLogFilePath();
        File.Delete(logFile);

        EasyLoggingOldDriver.Warn(logger, "This is a warning message");
        EasyLoggingOldDriver.Warn(logger, "This is another warning message");

        Assert.Equal(UnixMode.UserReadWriteGroupRead, EasyLoggingOldDriver.GetFilePermissions(logFile));
        var logs = File.ReadAllText(logFile);
        Assert.Contains("This is a warning message", logs);
        Assert.Contains("This is another warning message", logs);
    }

    [SnowflakeFact]
    public async Task ShouldReCreateEasyLoggingWindowsLogFileIgnoringCustomisedPermissionsAsync()
    {
        Skip.When(!RuntimeInformation.IsOSPlatform(OSPlatform.Windows), RuntimeInformation.IsOSPlatform(OSPlatform.Linux)
            ? "Test is skipped on Linux."
            : "Test is skipped on macOS.");
        Skip.OnNewDriver("BD#easy-logging-not-supported", EasyLoggingNotSupportedRationale);

        var configFilePath = CreateConfigTempFile(
            _classFixture.WorkingDirectory,
            Config("WARN", _classFixture.WorkingDirectory, "640"));
        using var conn = OpenConnectionWithConfig(configFilePath);
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        var logger = EasyLoggingOldDriver.GetLogger(typeof(EasyLoggingTest));
        var logFile = EasyLoggingOldDriver.FirstAppenderLogFilePath();
        File.Delete(logFile);

        EasyLoggingOldDriver.Warn(logger, "This is a warning message");
        EasyLoggingOldDriver.Warn(logger, "This is another warning message");

        var logs = File.ReadAllText(logFile);
        Assert.Contains("This is a warning message", logs);
        Assert.Contains("This is another warning message", logs);
    }

    [SnowflakeFact]
    public async Task ShouldFailToEnableEasyLoggingWhenConfigHasWrongPermissionsAsync()
    {
        Skip.When(RuntimeInformation.IsOSPlatform(OSPlatform.Windows), "Test is skipped on Windows.");
        Skip.OnNewDriver("BD#easy-logging-not-supported", EasyLoggingNotSupportedRationale);

        var configFilePath = CreateConfigTempFile(_classFixture.WorkingDirectory, Config("WARN", _classFixture.WorkingDirectory));
        Chmod(configFilePath, UnixMode.OwnerRwGroupWrite);
        using var conn = OpenConnectionWithConfig(configFilePath);
        var thrown = await AssertOpenThrowsSnowflakeDbExceptionAsync(conn, TestContext.Current.CancellationToken).ConfigureAwait(false);

        var messages = thrown.Message + "/n" + thrown.InnerException?.Message;
        Assert.Contains("Connection string is invalid: Unable to initialize session", messages);
        Assert.Empty(EasyLoggingOldDriver.GetAppenders());
    }

    [SnowflakeFact]
    public async Task ShouldFailToEnableEasyLoggingWhenLogDirectoryNotAccessibleAsync()
    {
        Skip.When(RuntimeInformation.IsOSPlatform(OSPlatform.Windows), "Test is skipped on Windows.");
        Skip.OnNewDriver("BD#easy-logging-not-supported", EasyLoggingNotSupportedRationale);

        var configFilePath = CreateConfigTempFile(_classFixture.WorkingDirectory, Config("WARN", "/"));
        using var conn = OpenConnectionWithConfig(configFilePath);
        var thrown = await AssertOpenThrowsSnowflakeDbExceptionAsync(conn, TestContext.Current.CancellationToken).ConfigureAwait(false);

        var message = FlattenExceptionMessages(thrown);
        Assert.Contains("Connection string is invalid: Unable to initialize session", message);
        Assert.Contains("Failed to create logs directory", message);
        Assert.Empty(EasyLoggingOldDriver.GetAppenders());
    }

    [SnowflakeFact]
    public async Task ShouldFailToEnableEasyLoggingWhenPathIsAccessibleForGroupAsync()
    {
        Skip.When(RuntimeInformation.IsOSPlatform(OSPlatform.Windows), "Test is skipped on Windows.");
        Skip.OnNewDriver("BD#easy-logging-not-supported", EasyLoggingNotSupportedRationale);

        var logDirectory = Path.Combine(_classFixture.WorkingDirectory, LogDirectoryName);
        Directory.CreateDirectory(logDirectory);
        Chmod(logDirectory, UnixMode.OwnerRwxGroupRead);

        var configFilePath = CreateConfigTempFile(
            _classFixture.WorkingDirectory,
            Config("WARN", _classFixture.WorkingDirectory, "640"));
        using var conn = OpenConnectionWithConfig(configFilePath);
        var thrown = await AssertOpenThrowsSnowflakeDbExceptionAsync(conn, TestContext.Current.CancellationToken).ConfigureAwait(false);

        var messages = FlattenExceptionMessages(thrown);
        Assert.Contains("Connection string is invalid: Unable to initialize session", messages);
        Assert.Contains("Too broad access permissions for logs directory", messages);
        Assert.Empty(EasyLoggingOldDriver.GetAppenders());
    }

    private ISnowflakeDbConnection OpenConnectionWithConfig(string configFilePath)
    {
        var baseConnectionString = Fixture.Factory.Create(Output).ConnectionString;
        return Fixture.Factory.Create(Output, AppendParam(baseConnectionString, $"CLIENT_CONFIG_FILE={configFilePath}"));
    }

    private static string AppendParam(string connectionString, string extra)
    {
        if (connectionString.EndsWith(";", StringComparison.Ordinal))
            return connectionString + extra;
        return connectionString + ";" + extra;
    }

    private static async Task<DbException> AssertOpenThrowsSnowflakeDbExceptionAsync(ISnowflakeDbConnection conn, CancellationToken ct)
    {
        var thrown = await Assert.ThrowsAnyAsync<DbException>(() => conn.OpenAsync(ct)).ConfigureAwait(false);
        AssertOldInvalidConnectionStringError(thrown);
        return thrown;
    }

    private static void AssertOldInvalidConnectionStringError(DbException thrown)
    {
        var old = Assert.IsType<OldSnowflakeDbException>(thrown);
        Assert.Equal(InvalidConnectionStringErrorCode, old.ErrorCode);
        Assert.Equal(ConnectionExceptionSqlState, old.SqlState);
    }

    private static string FlattenExceptionMessages(Exception thrown)
    {
        var sb = new StringBuilder();
        var current = thrown;
        while (current != null)
        {
            sb.Append(current.Message);
            current = current.InnerException;
        }

        return sb.ToString();
    }

    private static string CreateConfigTempFile(string workingDirectory, string fileContent)
    {
        var filePath = Path.Combine(workingDirectory, Path.GetRandomFileName());
        File.WriteAllText(filePath, fileContent);
        return filePath;
    }

    private static string Config(string logLevel, string logPath, string? logFileUnixPermissions = null)
    {
        var commonSection = $@"
                ""common"": {{
                    ""log_level"": {SerializeParameter(logLevel)},
                    ""log_path"": {SerializeParameter(logPath).Replace("\\", "\\\\")}
                }}";
        var dotnetSection = string.Empty;
        if (logFileUnixPermissions != null)
            dotnetSection = $@",
                ""dotnet"": {{
                    ""log_file_unix_permissions"": {SerializeParameter(logFileUnixPermissions)}
                }}";
        return "{" + commonSection + dotnetSection + "\n}";
    }

    private static string SerializeParameter(string? parameter) =>
        parameter == null ? "null" : $"\"{parameter}\"";

    [DllImport("libc", SetLastError = true)]
    private static extern int chmod(string pathname, uint mode);

    private static int Chmod(string path, uint mode) => chmod(path, mode);

    private static class UnixMode
    {
        internal const uint OwnerRwx = 0b111_000_000;
        internal const uint OwnerRwGroupWrite = 0b110_010_000;
        internal const uint OwnerRwxGroupRead = 0b111_100_000;
        internal const int UserReadWriteGroupRead = 0b110_100_000;
    }

}

internal static class EasyLoggingOldDriver
{
    internal static StarterState CaptureStarterState()
    {
        var starter = StarterInstance();
        return new StarterState(
            StarterField("_initTrialParameters").GetValue(starter),
            StarterField("_logFileUnixPermissions").GetValue(starter),
            LoggerLevelField().GetValue(null));
    }

    // The old starter refuses every later Init once any connection tried a config file path.
    internal static void ClearInitTrial() =>
        StarterField("_initTrialParameters").SetValue(StarterInstance(), null);

    internal static void RestoreStarterState(StarterState state)
    {
        var starter = StarterInstance();
        StarterField("_initTrialParameters").SetValue(starter, state.InitTrialParameters);
        StarterField("_logFileUnixPermissions").SetValue(starter, state.LogFileUnixPermissions);
        RequireType("Snowflake.Data.Log.SFLoggerImpl")
            .GetMethod("SetLevel", BindingFlags.Static | BindingFlags.NonPublic | BindingFlags.Public)!
            .Invoke(null, [state.LoggerLevel]);
    }

    internal static bool HasEasyLoggingAppender()
    {
        var managerType = RequireType("Snowflake.Data.Log.EasyLoggerManager");
        return (bool)managerType.GetMethod(
                "HasEasyLoggingAppender",
                BindingFlags.Static | BindingFlags.NonPublic | BindingFlags.Public)!
            .Invoke(null, null)!;
    }

    internal static object? CaptureAppenders()
    {
        var field = AppendersField();
        var current = field.GetValue(null);
        if (current is not System.Collections.IEnumerable enumerable)
            return null;

        var copy = CreateEmptyAppenderList();
        foreach (var item in enumerable)
            ((System.Collections.IList)copy).Add(item);
        return copy;
    }

    internal static object CreateEmptyAppenderList()
    {
        var appenderType = RequireType("Snowflake.Data.Log.SFAppender");
        var listType = typeof(List<>).MakeGenericType(appenderType);
        return Activator.CreateInstance(listType)!;
    }

    internal static void ReplaceAppenders(object? appenders)
    {
        if (appenders == null)
            return;
        AppendersField().SetValue(null, appenders);
    }

    internal static System.Collections.IList GetAppenders() =>
        (System.Collections.IList)AppendersField().GetValue(null)!;

    internal static object GetLogger(Type type)
    {
        var factoryType = RequireType("Snowflake.Data.Log.SFLoggerFactory");
        var method = factoryType.GetMethod(
            "GetSFLogger",
            BindingFlags.Static | BindingFlags.NonPublic | BindingFlags.Public,
            null,
            Type.EmptyTypes,
            null);
        if (method != null)
            return method.MakeGenericMethod(type).Invoke(null, null)!;

        var generic = factoryType.GetMethods(BindingFlags.Static | BindingFlags.NonPublic | BindingFlags.Public)
            .First(m => m.Name == "GetSFLogger" && m.IsGenericMethodDefinition);
        return generic.MakeGenericMethod(type).Invoke(null, null)!;
    }

    internal static string FirstAppenderLogFilePath()
    {
        var appender = GetAppenders()[0]!;
        var path = appender.GetType()
            .GetProperty("LogFilePath", BindingFlags.Instance | BindingFlags.NonPublic | BindingFlags.Public)
            ?.GetValue(appender);
        return Assert.IsType<string>(path);
    }

    internal static void Warn(object logger, string message)
    {
        logger.GetType().GetMethod("Warn", BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)!
            .Invoke(logger, [message, null]);
    }

    internal static int GetFilePermissions(string logFile)
    {
        var unixOpsType = RequireType("Snowflake.Data.Core.Tools.UnixOperations");
        var instance = unixOpsType.GetField("Instance", BindingFlags.Public | BindingFlags.Static)!.GetValue(null);
        var permissions = unixOpsType.GetMethod(
                "GetFilePermissions",
                BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)!
            .Invoke(instance, [logFile]);
        return Convert.ToInt32(permissions, CultureInfo.InvariantCulture);
    }

    private static object StarterInstance() =>
        RequireType("Snowflake.Data.Core.EasyLoggingStarter")
            .GetField("Instance", BindingFlags.Public | BindingFlags.Static)!
            .GetValue(null)!;

    private static FieldInfo StarterField(string name) =>
        RequireType("Snowflake.Data.Core.EasyLoggingStarter")
            .GetField(name, BindingFlags.Instance | BindingFlags.NonPublic | BindingFlags.Public)!;

    private static FieldInfo LoggerLevelField() =>
        RequireType("Snowflake.Data.Log.SFLoggerImpl")
            .GetField("s_level", BindingFlags.Static | BindingFlags.NonPublic | BindingFlags.Public)!;

    private static FieldInfo AppendersField()
    {
        var loggerType = RequireType("Snowflake.Data.Log.SFLoggerImpl");
        return loggerType.GetField(
            "s_appenders",
            BindingFlags.Static | BindingFlags.NonPublic | BindingFlags.Public)!;
    }

    private static Type RequireType(string fullName)
    {
        var type = typeof(OldSnowflakeDbConnection).Assembly.GetType(fullName);
        Assert.NotNull(type);
        return type;
    }

    internal sealed class StarterState
    {
        internal StarterState(object? initTrialParameters, object? logFileUnixPermissions, object? loggerLevel)
        {
            InitTrialParameters = initTrialParameters;
            LogFileUnixPermissions = logFileUnixPermissions;
            LoggerLevel = loggerLevel;
        }

        internal object? InitTrialParameters { get; }

        internal object? LogFileUnixPermissions { get; }

        internal object? LoggerLevel { get; }
    }
}
