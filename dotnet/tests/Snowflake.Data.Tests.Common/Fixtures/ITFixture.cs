using Snowflake.Data.Tests.Utilities;

namespace Snowflake.Data.Tests.Fixtures;

public class ITFixture
#if !NETFRAMEWORK
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

    public virtual ITestConnectionFactory Factory
    {
        get => OverriddenFactory ?? field;
    } = new DefaultTestConnectionFactory();

    public static ITestConnectionFactory? OverriddenFactory { get; set; }

    private static ITestOutputHelper? TestOutputHelper => TestContext.Current.TestOutputHelper;

    static ITFixture()
    {
        InitializeEnvironment();
    }

#if !NETFRAMEWORK
    public virtual async ValueTask InitializeAsync()
    {
        var suffix = Guid.NewGuid().ToString("N")[..8];
        _schemaName = $"{_baseSchema}_DOTNET_{suffix}";

        try
        {
            await DDLCallAsync($"CREATE SCHEMA {_schemaName}").ConfigureAwait(false);
            await DDLCallAsync($"USE SCHEMA {_schemaName}").ConfigureAwait(false);
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
            await DDLCallAsync($"DROP SCHEMA {_schemaName}").ConfigureAwait(false);
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
            DDLCallAsync($"DROP IF EXISTS SCHEMA {_schemaName}").GetAwaiter().GetResult();
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
            DDLCallAsync($"CREATE OR REPLACE SCHEMA {_schemaName}").GetAwaiter().GetResult();
        }
        catch(Exception ex)
        {
            Console.WriteLine($"[Fixture] Schema creation failed: {ex.Message}");
            throw;
        }
    }
#endif

    private async Task DDLCallAsync(string ddl)
    {
        using var connection = Factory.Create(TestOutputHelper);

        await connection.OpenAsync().ConfigureAwait(false);

        using var cmd = connection.CreateCommand();
        cmd.CommandText = ddl;

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
