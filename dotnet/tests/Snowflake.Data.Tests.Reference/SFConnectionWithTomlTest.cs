using System.Data.Common;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.Reference.Fixtures;
using NewSnowflakeDbException = Snowflake.Data.SnowflakeDbException;
using OldSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.Reference;

[Collection(nameof(SFConnectionWithTomlTest))]
public sealed class SFConnectionWithTomlTest : ReferenceTestBase, IDisposable
{
    private const string SnowflakeHome = "SNOWFLAKE_HOME";
    private const string DefaultConnectionName = "SNOWFLAKE_DEFAULT_CONNECTION_NAME";
    private const int InternalErrorCode = 270001;
    private const int MissingRequiredPropertyErrorCode = 270006;
    private const string ConnectionFailureSqlState = "08006";

    private readonly string _workingDirectory =
        Path.Combine(Path.GetTempPath(), $"toml_config_folder_{Guid.NewGuid():N}");

    public SFConnectionWithTomlTest(ReferenceITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
        Directory.CreateDirectory(_workingDirectory);
        CreateTomlConfigBaseOnConnectionString(BaseConnectionString());
    }

    public void Dispose()
    {
        Directory.Delete(_workingDirectory, true);
    }

    [SnowflakeFact]
    public async Task ShouldOpenDefaultConnectionReadFromLocalTomlAsync()
    {
        // TODO(SNOW-4382580): sf_core resolves SNOWFLAKE_HOME and SNOWFLAKE_DEFAULT_CONNECTION_NAME from the native
        // process environment, which Environment.SetEnvironmentVariable does not update on Unix
        Skip.FutureMilestone();

        var snowflakeHome = Environment.GetEnvironmentVariable(SnowflakeHome);
        Environment.SetEnvironmentVariable(SnowflakeHome, _workingDirectory);
        try
        {
            using var conn = Fixture.Factory.Create(Output, string.Empty);
            await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.Equal(ConnectionState.Open, conn.State);
        }
        finally
        {
            Environment.SetEnvironmentVariable(SnowflakeHome, snowflakeHome);
        }
    }

    [SnowflakeFact]
    public async Task ShouldThrowWhenDefaultConnectionNameNotFoundInTomlAsync()
    {
        // TODO(SNOW-4382580): sf_core resolves SNOWFLAKE_HOME and SNOWFLAKE_DEFAULT_CONNECTION_NAME from the native
        // process environment, which Environment.SetEnvironmentVariable does not update on Unix
        Skip.FutureMilestone();

        var snowflakeHome = Environment.GetEnvironmentVariable(SnowflakeHome);
        var connectionName = Environment.GetEnvironmentVariable(DefaultConnectionName);
        Environment.SetEnvironmentVariable(SnowflakeHome, _workingDirectory);
        Environment.SetEnvironmentVariable(DefaultConnectionName, "notfoundconnection");
        try
        {
            using var conn = Fixture.Factory.Create(Output, string.Empty);
            var thrown = await Assert.ThrowsAnyAsync<DbException>(
                () => conn.OpenAsync(TestContext.Current.CancellationToken)).ConfigureAwait(false);
            AssertIsSnowflakeDbException(thrown, InternalErrorCode, ConnectionFailureSqlState);
        }
        finally
        {
            Environment.SetEnvironmentVariable(SnowflakeHome, snowflakeHome);
            Environment.SetEnvironmentVariable(DefaultConnectionName, connectionName);
        }
    }

    [SnowflakeFact]
    public async Task ShouldThrowWhenTomlFileNotFoundAsync()
    {
        // TODO(SNOW-4382580): sf_core resolves SNOWFLAKE_HOME and SNOWFLAKE_DEFAULT_CONNECTION_NAME from the native
        // process environment, which Environment.SetEnvironmentVariable does not update on Unix
        Skip.FutureMilestone();

        var snowflakeHome = Environment.GetEnvironmentVariable(SnowflakeHome);
        Environment.SetEnvironmentVariable(SnowflakeHome, Path.Combine(_workingDirectory, "InvalidFolder"));
        try
        {
            using var conn = Fixture.Factory.Create(Output, string.Empty);
            var thrown = await Assert.ThrowsAnyAsync<DbException>(
                () => conn.OpenAsync(TestContext.Current.CancellationToken)).ConfigureAwait(false);
            AssertIsSnowflakeDbException(thrown, MissingRequiredPropertyErrorCode, null);
        }
        finally
        {
            Environment.SetEnvironmentVariable(SnowflakeHome, snowflakeHome);
        }
    }

    private static void AssertIsSnowflakeDbException(Exception exception, int expectedErrorCode, string? expectedSqlState)
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
        {
            var old = Assert.IsType<OldSnowflakeDbException>(exception);
            Assert.Equal(expectedErrorCode, old.ErrorCode);
            Assert.Equal(expectedSqlState, old.SqlState);
        }
        else
        {
            var current = Assert.IsType<NewSnowflakeDbException>(exception);
            Assert.Equal(expectedErrorCode, current.ErrorCode);
            Assert.Equal(expectedSqlState, current.SqlState);
        }
    }

    private string BaseConnectionString()
    {
        using var connection = Fixture.Factory.Create(Output);
        return connection.ConnectionString;
    }

    private void CreateTomlConfigBaseOnConnectionString(string connectionString)
    {
        var toml = new StringBuilder("[default]\n");
        foreach (var property in connectionString.Split(new[] { ';' }, StringSplitOptions.RemoveEmptyEntries))
        {
            var separator = property.IndexOf('=');
            var key = property.Substring(0, separator);
            var value = property.Substring(separator + 1).Replace("\\", "\\\\").Replace("\"", "\\\"");
            toml.Append(key).Append(" = \"").Append(value).Append("\"\n");
        }

        var filePath = Path.Combine(_workingDirectory, "connections.toml");
        File.WriteAllText(filePath, toml.ToString());
        if (!RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            using var chmod = Process.Start("chmod", $"600 \"{filePath}\"");
            chmod.WaitForExit();
            Assert.Equal(0, chmod.ExitCode);
        }
    }

    [CollectionDefinition(nameof(SFConnectionWithTomlTest), DisableParallelization = true)]
    public sealed class SerialCollection
    {
    }
}
