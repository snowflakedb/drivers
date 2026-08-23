using Snowflake.Data.Tests.Utilities;

namespace Snowflake.Data.Tests.Fixtures;

public class ITFixture
#if !OLD_XUNIT
    : IAsyncLifetime
#else
    : IDisposable
#endif
{
    private const string SFCoreLibPath = "SF_CORE_LIB_PATH";
    private const string SFCore = "sf_core";
    private const string SnowflakeTestSchema = "SNOWFLAKE_TEST_SCHEMA";

    private static string? _baseSchema;
    private string? _schemaName;
    public virtual ITestConnectionFactory Factory { get; } = new DefaultTestConnectionFactory();

    private static ITestOutputHelper? TestOutputHelper =>
#if OLD_XUNIT
        NullTestOutputHelper.Instance;
#else
        TestContext.Current.TestOutputHelper;
#endif

    static ITFixture()
    {
        InitializeEnvironment();
    }

#if !OLD_XUNIT
    public async ValueTask InitializeAsync()
    {
        var suffix = Guid.NewGuid().ToString("N")[..8];
        _schemaName = $"{_baseSchema}_DOTNET_{suffix}";

        try
        {
            await ModifySchemaAsync(_schemaName, "CREATE OR REPLACE").ConfigureAwait(false);
        }
        catch (Exception ex)
        {
            Console.WriteLine($"[Fixture] Schema creation failed: {ex.Message}");
            throw;
        }
    }

    public async ValueTask DisposeAsync()
    {
        if (_schemaName is null)
            return;

        try
        {
            await ModifySchemaAsync(_schemaName, "DROP").ConfigureAwait(false);
        }
        catch (Exception ex)
        {
            Console.WriteLine($"[Fixture] Schema cleanup failed: {ex.Message}");
        }
    }
#else
    public void Dispose()
    {
        if (_schemaName is null)
            return;

        try
        {
            ModifySchemaAsync(_schemaName, "DROP IF EXISTS").GetAwaiter().GetResult();
        }
        catch(Exception ex)
        {
            TestOutputHelper?.WriteLine($"[Fixture] Schema cleanup failed {ex.Message}");
        }
    }

    public ITFixture()
    {
        var suffix = Guid.NewGuid().ToString("N").Substring(0, 8);
        _schemaName = $"{_baseSchema}_DOTNET_{suffix}";

        try
        {
            ModifySchemaAsync(_schemaName, "CREATE OR REPLACE").GetAwaiter().GetResult();
        }
        catch(Exception ex)
        {
            Console.WriteLine($"[Fixture] Schema creation failed: {ex.Message}");
            throw;
        }
    }
#endif

    private async Task ModifySchemaAsync(string schemaName, string action)
    {
        using var connection = Factory.Create(TestOutputHelper);

        await connection.OpenAsync().ConfigureAwait(false);

        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"{action} SCHEMA {schemaName}";

        await cmd.ExecuteNonQueryAsync().ConfigureAwait(false);
    }

    private static void InitializeEnvironment()
    {
        var currentDirectory = Directory.GetCurrentDirectory();

        if (Env.ConfigurationMode != ConfigurationMode.Debug && string.IsNullOrEmpty(Environment.GetEnvironmentVariable(SFCoreLibPath)))
            throw new InvalidOperationException($"{SFCoreLibPath} env variable must be set!");

        var currentDirectoryPath = Path.GetFullPath(currentDirectory);
        var nestedDirCount = 0;
        for (; ; )
        {
            var dirs = Directory.EnumerateDirectories(currentDirectoryPath).Select(x => new DirectoryInfo(x).Name);
            if (dirs.Contains(SFCore))
            {
                var fullPath = Path.Combine(currentDirectoryPath, "target/debug");
                fullPath = Path.GetFullPath(fullPath);
                Environment.SetEnvironmentVariable(SFCoreLibPath, fullPath);
                break;
            }

            currentDirectoryPath = new DirectoryInfo(currentDirectoryPath).Parent?.FullName ?? string.Empty;

            if (nestedDirCount++ > 10)
                throw new InvalidOperationException($"Did not find root directory after stepping 10 levels!");
        }

        ITEnvironment.Init();

        _baseSchema = ParametersReader.Get(SnowflakeTestSchema);
        if (_baseSchema != null)
            return;

        TestOutputHelper?.WriteLine($"[Fixture] Schema setting not found. Will fallback to default one..");
        _baseSchema = "PUBLIC";
    }
}

#if OLD_XUNIT
file sealed class NullTestOutputHelper : ITestOutputHelper
{
    public static NullTestOutputHelper Instance { get; } = new();
    public string Output => string.Empty;
    public void Write(string message) { }
    public void Write(string format, params object[] args) { }
    public void WriteLine(string message) { }
    public void WriteLine(string format, params object[] args) { }
}
#endif
