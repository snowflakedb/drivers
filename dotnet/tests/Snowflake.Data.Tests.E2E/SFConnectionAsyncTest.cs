extern alias OldDriver;
using System.Data.Common;
using System.Globalization;
using System.Runtime.InteropServices;
using System.Security;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E.Fixtures;
using Snowflake.Data.Tests.Utilities;
using NewSnowflakeDbConnection = Snowflake.Data.Client.SnowflakeDbConnection;
using NewSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;
using OldSnowflakeDbConnection = OldDriver::Snowflake.Data.Client.SnowflakeDbConnection;
using OldSnowflakeDbException = OldDriver::Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.E2E;

public sealed class SFConnectionAsyncTest : LiveE2ETestBase
{
    private const string ConnectionFailureSqlState = "08006";
    private const int InternalErrorCode = 270001;
    private const int ObjectNonexistentOrNotAuthorizedErrorCode = 390201;
    private const int ExtOAuthAccessTokenInvalidErrorCode = 390303;
    private const int OldDriverDefaultRetryTimeoutSeconds = 300;

    private static readonly string[] s_authenticationKeys =
        ["user", "password", "token", "authenticator", "private_key_file", "private_key", "private_key_pwd"];

    public SFConnectionAsyncTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeFact]
    public async Task ShouldOpenBasicConnectionAsync()
    {
        // TODO(SNOW-4382580): ConnectionTimeout defaults to the DbConnection base value and DataSource/ServerVersion are placeholders
        Skip.FutureMilestone();

        using var conn = CreateDbConnection(BaseConnectionString());
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn.State);

        Assert.Equal(OldDriverDefaultRetryTimeoutSeconds, conn.ConnectionTimeout);
        Assert.Equal("", conn.DataSource);

        var serverVersion = conn.ServerVersion;
        if (!string.IsNullOrEmpty(serverVersion))
        {
            var versionElements = serverVersion.Split('.');
            Assert.Equal(3, versionElements.Length);
        }

        await CloseDbConnectionAsync(conn, TestContext.Current.CancellationToken).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Closed, conn.State);
    }

    [SnowflakeFact]
    public async Task ShouldValidateApplicationNameAsync()
    {
        // TODO(SNOW-4382580): invalid application names in the connection string are not rejected
        Skip.FutureMilestone();

        string[] validApplicationNames = ["test1234", "test_1234", "test-1234", "test.1234"];
        string[] invalidApplicationNames = ["1234test", "test$A", "test<script>"];

        foreach (var appName in validApplicationNames)
        {
            using var conn = CreateConnection(BaseConnectionString() + $";application={appName}");
            await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.Equal(ConnectionState.Open, conn.State);

            await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.Equal(ConnectionState.Closed, conn.State);
        }

        foreach (var appName in invalidApplicationNames)
        {
            using var conn = CreateConnection(BaseConnectionString() + $";application={appName}");
            var exception = await Assert.ThrowsAnyAsync<DbException>(
                () => conn.OpenAsync(TestContext.Current.CancellationToken)).ConfigureAwait(false);

            AssertIsConnectionFailure(exception);
            Assert.Equal(ConnectionState.Closed, conn.State);
        }
    }

    [SnowflakeFact]
    public async Task ShouldFailBasicConnectionWithIncorrectUserOrPasswordAsync()
    {
        // TODO(SNOW-4382591): login failures surface as ServiceException, not SnowflakeDbException with SqlState
        Skip.FutureMilestone();

        using var conn = CreateConnection(ConnectionStringWithInvalidUserName());
        Assert.Equal(ConnectionState.Closed, conn.State);

        var exception = await Assert.ThrowsAnyAsync<DbException>(
            () => conn.OpenAsync(TestContext.Current.CancellationToken)).ConfigureAwait(false);

        AssertIsConnectionFailure(exception);
        Assert.Equal(ConnectionState.Closed, conn.State);
    }

    [SnowflakeTheory]
    [InlineData(true)]
    [InlineData(false)]
    public async Task ShouldNotMarkConnectionAsOpenWhenItWasNotCorrectlyOpenedBeforeAsync(bool explicitClose)
    {
        // TODO(SNOW-4382591): login failures surface as ServiceException, not SnowflakeDbException with SqlState
        Skip.FutureMilestone();

        for (var i = 0; i < 2; ++i)
        {
            using var conn = CreateConnection(ConnectionStringWithInvalidUserName());
            var exception = await Assert.ThrowsAnyAsync<DbException>(
                () => conn.OpenAsync(TestContext.Current.CancellationToken)).ConfigureAwait(false);

            AssertIsConnectionFailure(exception);
            AssertConnectionIsNotOpen(conn);
            if (explicitClose)
            {
                await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
                AssertConnectionIsNotOpen(conn);
            }
        }
    }

    [SnowflakeFact]
    public async Task ShouldNotMarkConnectionAsOpenWhenItWasNotCorrectlyOpenedWithUsingClauseAsync()
    {
        // TODO(SNOW-4382591): login failures surface as ServiceException, not SnowflakeDbException with SqlState
        Skip.FutureMilestone();

        for (var i = 0; i < 2; ++i)
        {
            ISnowflakeDbConnection? conn = null;
            try
            {
                using (conn = CreateConnection(ConnectionStringWithInvalidUserName()))
                {
                    await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
                }
            }
            catch (DbException exception)
            {
                AssertIsConnectionFailure(exception);
                AssertConnectionIsNotOpen(conn);
            }
        }
    }

    [SnowflakeFact]
    public async Task ShouldUseSchemaFromConnectStringAsync()
    {
        var tableName = $"CONNECTION_ASYNC_TEST_{Guid.NewGuid():N}";
        var schemaName = $"DLSCHEMA_{Guid.NewGuid():N}".ToUpperInvariant();
        var baseConnectionString = BaseConnectionString();
        var database = Setting(baseConnectionString, "database");
        var schema = Setting(baseConnectionString, "schema");
        using var conn = CreateConnection(baseConnectionString);
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            using (var cmd = conn.CreateCommand())
            {
                cmd.CommandText = $"create schema \"{schemaName}\"";
                await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
                cmd.CommandText = $"use schema \"{schemaName}\"";
                await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
                cmd.CommandText = $"create table {tableName} (col1 string, col2 int)";
                await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
                cmd.CommandText = $"insert into {tableName} Values ('test 1', 1);";
                await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            }

            using (var conn1 = CreateConnection(baseConnectionString.Replace($"schema={schema}", $"schema={schemaName}")))
            {
                Assert.Equal(ConnectionState.Closed, conn1.State);

                await conn1.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
                using (IDbCommand cmd = conn1.CreateCommand())
                {
                    cmd.CommandText = $"SELECT count(*) FROM {tableName}";
                    using var reader = cmd.ExecuteReader();
                    Assert.True(reader.Read());
                    Assert.Equal(1, reader.GetInt32(0));
                }

                await conn1.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
                Assert.Equal(ConnectionState.Closed, conn1.State);
            }
        }
        finally
        {
            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"drop schema if exists \"{schemaName}\"";
            await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            cmd.CommandText = "use database " + database;
            await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            cmd.CommandText = "use schema " + schema;
            await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldConnectViaSecureStringAsync()
    {
        // TODO(SNOW-4382580): the Password setter throws NotImplementedException
        Skip.FutureMilestone();

        var connEntries = BaseConnectionString().Split(';');
        var connectionStringWithoutPassword = "";
        var password = new SecureString();
        foreach (var entry in connEntries)
        {
            if (!entry.StartsWith("password=", StringComparison.Ordinal))
            {
                connectionStringWithoutPassword += entry;
                connectionStringWithoutPassword += ';';
            }
            else
            {
                foreach (var c in entry.Substring(9))
                    password.AppendChar(c);
            }
        }

        using var conn = CreateConnection(connectionStringWithoutPassword);
        conn.Password = password;
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(Setting(BaseConnectionString(), "database").ToUpperInvariant(), conn.Database);
        Assert.Equal(ConnectionState.Open, conn.State);

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldValidateDefaultParametersAsync()
    {
        // TODO(SNOW-4382580): VALIDATE_DEFAULT_PARAMETERS is not honored; a nonexistent warehouse does not fail the login
        Skip.FutureMilestone();

        var baseConnectionString = BaseConnectionString();
        var warehouse = Setting(baseConnectionString, "warehouse");
        var connectionString = baseConnectionString.Replace($"warehouse={warehouse}", "warehouse=WAREHOUSE_NEVER_EXISTS");

        using (var conn = CreateConnection(connectionString))
        {
            var exception = await Assert.ThrowsAnyAsync<DbException>(
                () => conn.OpenAsync(TestContext.Current.CancellationToken)).ConfigureAwait(false);

            AssertSnowflakeDbException(exception);
            var aggregate = Assert.IsType<AggregateException>(exception.InnerException);
            Assert.Equal(
                ObjectNonexistentOrNotAuthorizedErrorCode,
                AssertSnowflakeDbException(aggregate.InnerExceptions[0]).ErrorCode);
        }

        using var conn2 = CreateConnection(connectionString + ";VALIDATE_DEFAULT_PARAMETERS=false");
        await conn2.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldIgnoreUnknownConnectionPropertyAsync()
    {
        using var conn = CreateConnection(BaseConnectionString() + ";invalidProperty=invalidvalue;");

        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn.State);
        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldSwitchDbAsync()
    {
        Skip.For(SkipCondition.SkipOnCloudAzure | SkipCondition.SkipOnCloudGCP, "Test is skipped on Azure and GCP.");
        // TODO(SNOW-4382580): Database returns an empty string and ChangeDatabase throws NotSupportedException
        Skip.FutureMilestone();

        var database = Setting(BaseConnectionString(), "database");
        using var conn = CreateConnection(BaseConnectionString());
        Assert.Equal(ConnectionState.Closed, conn.State);

        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(database.ToUpperInvariant(), conn.Database);
        Assert.Equal(ConnectionState.Open, conn.State);

        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            conn.ChangeDatabase("SNOWFLAKE_SAMPLE_DATA");
            Assert.Equal("SNOWFLAKE_SAMPLE_DATA", conn.Database);
        }

        conn.ChangeDatabase(database);
        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeTheory]
    [InlineData("SNOWFLAKE_SAMPLE_DAT")]
    [InlineData("SNOWFLAKE_$$$123SAMPLE_DATA")]
    [InlineData("\"1SNOWFLAKE_SAMPLE_DATA\"")]
    [InlineData("_SNOWFLAKE$$1_$AMPLEDATA")]
    [InlineData("$SNOWFLAKEAMP_LEDATA")]
    [InlineData("$SNOWFLAKEAMP_12LEDATA")]
    [InlineData("\"SNOWFLAKEAMPLEDATA\"")]
    [InlineData("\"SNOWFLAKE\"\"AMPLEDATA\"")]
    public async Task ShouldFailToSwitchDbForVariousIdentifiersAsync(string databaseName)
    {
        Skip.For(SkipCondition.RunOnlyOnCloudAWS, "Test runs only on AWS.");
        // TODO(SNOW-4382580): Database returns an empty string and ChangeDatabase throws NotSupportedException
        Skip.FutureMilestone();

        var database = Setting(BaseConnectionString(), "database");
        using var conn = CreateDbConnection(BaseConnectionString());
        Assert.Equal(ConnectionState.Closed, conn.State);

        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(database.ToUpperInvariant(), conn.Database);
        Assert.Equal(ConnectionState.Open, conn.State);

        var exception = await Assert.ThrowsAnyAsync<DbException>(
            () => conn.ChangeDatabaseAsync(databaseName)).ConfigureAwait(false);
        Assert.Equal(2043, AssertSnowflakeDbException(exception).ErrorCode);
        await CloseDbConnectionAsync(conn, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeTheory]
    [InlineData("@SNOWFLAKE_SAMPLE_DATA")]
    [InlineData("!SNOWFLAKE_SAMPLE_DATA")]
    [InlineData("1SNOWFLAKE_SAMPLE_DATA")]
    [InlineData("\"SNOW\"FLAKE_SAMPLE_DATA\"")]
    public async Task ShouldFailToSwitchDbWhenInvalidIdentifierAsync(string invalidDatabaseName)
    {
        Skip.For(SkipCondition.RunOnlyOnCloudAWS, "Test runs only on AWS.");
        // TODO(SNOW-4382580): Database returns an empty string and ChangeDatabase throws NotSupportedException
        Skip.FutureMilestone();

        var database = Setting(BaseConnectionString(), "database");
        using var conn = CreateDbConnection(BaseConnectionString());
        Assert.Equal(ConnectionState.Closed, conn.State);

        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(database.ToUpperInvariant(), conn.Database);
        Assert.Equal(ConnectionState.Open, conn.State);

        var exception = await Assert.ThrowsAnyAsync<DbException>(
            () => conn.ChangeDatabaseAsync(invalidDatabaseName)).ConfigureAwait(false);
        Assert.Equal(1003, AssertSnowflakeDbException(exception).ErrorCode);
        await CloseDbConnectionAsync(conn, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithoutHostAsync()
    {
        var baseConnectionString = BaseConnectionString();
        var connectionString = TryGetSetting(baseConnectionString, "host", out var host)
            ? baseConnectionString.Replace($"host={host}", string.Empty)
            : baseConnectionString;
        using var conn = CreateConnection(connectionString);

        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn.State);
        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithDifferentRoleAsync()
    {
        Skip.When(
            true,
            "The test account authenticates with a programmatic access token bound to its role; logging in with role=public is rejected with 390186");

        var baseConnectionString = BaseConnectionString();
        var role = Setting(baseConnectionString, "role");
        var database = Setting(baseConnectionString, "database");
        var schema = Setting(baseConnectionString, "schema");
        var warehouse = Setting(baseConnectionString, "warehouse");
        using var conn = CreateConnection(baseConnectionString.Replace($"role={role}", "role=public"));
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn.State);

        using (var command = conn.CreateCommand())
        {
            command.CommandText = "select current_role()";
            Assert.Equal("PUBLIC", (await command.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))!.ToString());

            command.CommandText = "select current_database()";
            Assert.Contains(
                (await command.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))!.ToString(),
                new[] { database, "" });

            command.CommandText = "select current_schema()";
            Assert.Contains(
                (await command.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))!.ToString(),
                new[] { schema.ToUpperInvariant(), "" });

            command.CommandText = "select current_warehouse()";
            Assert.Equal(warehouse, (await command.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))!.ToString());
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldRollbackUncommittedTransactionOnConnectionDisposeAsync()
    {
        // TODO(SNOW-4382589): BeginTransaction throws NotImplementedException
        Skip.FutureMilestone();

        var tableName = $"CONNECTION_ASYNC_TEST_{Guid.NewGuid():N}";
        try
        {
            using (var conn = CreateDbConnection(BaseConnectionString()))
            {
                await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
                using (var create = conn.CreateCommand())
                {
                    create.CommandText = $"CREATE OR REPLACE TABLE {tableName}(c INT)";
                    await create.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
                }

                var t1 = await conn.BeginTransactionAsync().ConfigureAwait(false);
                var t1c1 = conn.CreateCommand();
                t1c1.Transaction = t1;
                t1c1.CommandText = $"insert into {tableName} values (1)";
                await t1c1.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            }

            using (var conn = CreateConnection(BaseConnectionString()))
            {
                await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
                using var command = conn.CreateCommand();
                command.CommandText = $"SELECT * FROM {tableName}";
                using var reader = await command.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
                Assert.False(reader.Read());
            }
        }
        finally
        {
            DropTable(tableName);
        }
    }

    [SnowflakeFact]
    public async Task ShouldRejectInvalidOAuthTokenConnectionAsync()
    {
        // TODO(SNOW-4382591): login failures surface as ServiceException, not SnowflakeDbException
        Skip.FutureMilestone();

        using var conn = CreateConnection($"{BaseConnectionStringWithoutAuth()};authenticator=oauth;token=notAValidOAuthToken");

        var exception = await Assert.ThrowsAnyAsync<DbException>(
            () => conn.OpenAsync(TestContext.Current.CancellationToken)).ConfigureAwait(false);

        AssertSnowflakeDbException(exception);
        var aggregate = Assert.IsType<AggregateException>(exception.InnerException);
        Assert.Equal(ExtOAuthAccessTokenInvalidErrorCode, AssertSnowflakeDbException(aggregate.InnerException!).ErrorCode);
    }

    [SnowflakeFact]
    public async Task ShouldFailWithInvalidProxySettingFromConnectionStringAsync()
    {
        // TODO(SNOW-4382580): useProxy/proxyHost/proxyPort connection-string options are ignored
        Skip.FutureMilestone();

        using var conn = CreateConnection(
            BaseConnectionString() + ";connection_timeout=5;useProxy=true;proxyHost=Invalid;proxyPort=8080");

        var exception = await Assert.ThrowsAnyAsync<DbException>(
            () => conn.OpenAsync(TestContext.Current.CancellationToken)).ConfigureAwait(false);

        Assert.Equal(InternalErrorCode, AssertSnowflakeDbException(exception).ErrorCode);
        AssertIsConnectionFailure(exception);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.Thrice)]
    [InlineData("*")]
    [InlineData("*{0}*")]
    [InlineData("^*{0}*")]
    [InlineData("*{0}*$")]
    [InlineData("^*{0}*$")]
    [InlineData("^nonmatch*{0}$|*")]
    [InlineData("*a*", "a")]
    [InlineData("*la*", "la")]
    public async Task ShouldBypassProxyServerForNonProxyHostAsync(string regexHost, string proxyHost = "proxyserverhost")
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
        {
            Skip.For(
                SkipCondition.SkipOnCI,
                "On the GitHub Actions Linux runner the old driver intermittently sends the login request to the proxy despite the bypass list and retries until the connection timeout");
        }

        // TODO(SNOW-4382580): useProxy/proxyHost/proxyPort/nonProxyHosts connection-string options are ignored
        Skip.FutureMilestone();

        var nonProxyHosts = string.Format(CultureInfo.InvariantCulture, regexHost, ResolveHost());
        using var conn = CreateConnection(
            $"{BaseConnectionString()};USEPROXY=true;PROXYHOST={proxyHost};NONPROXYHOSTS={nonProxyHosts};PROXYPORT=3128;");

        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(ConnectionState.Open, conn.State);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.Thrice)]
    [InlineData("invalid{0}")]
    [InlineData("*invalid{0}*")]
    [InlineData("^invalid{0}$")]
    [InlineData("*a.b")]
    [InlineData("a", "a")]
    [InlineData("la", "la")]
    public async Task ShouldNotBypassProxyServerForNonMatchingNonProxyHostAsync(string regexHost, string proxyHost = "proxyserverhost")
    {
        // TODO(SNOW-4382580): useProxy/proxyHost/proxyPort/nonProxyHosts connection-string options are ignored
        Skip.FutureMilestone();

        var nonProxyHosts = string.Format(CultureInfo.InvariantCulture, regexHost, ResolveHost());
        using var conn = CreateConnection(
            $"{BaseConnectionString()};connection_timeout=5;USEPROXY=true;PROXYHOST={proxyHost};NONPROXYHOSTS={nonProxyHosts};PROXYPORT=3128;");

        var exception = await Assert.ThrowsAnyAsync<DbException>(
            () => conn.OpenAsync(TestContext.Current.CancellationToken)).ConfigureAwait(false);

        Assert.Equal(InternalErrorCode, AssertSnowflakeDbException(exception).ErrorCode);
        AssertIsConnectionFailure(exception);
    }

    [SnowflakeFact]
    public async Task ShouldIgnoreInvalidProxyWhenUseProxyIsFalseAsync()
    {
        using var conn = CreateConnection(BaseConnectionString() + ";useProxy=false;proxyHost=Invalid;proxyPort=8080");

        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(ConnectionState.Open, conn.State);
    }

    [SnowflakeFact]
    public async Task ShouldBypassInvalidProxyWithByPassListFromConnectionStringAsync()
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
        {
            Skip.For(
                SkipCondition.SkipOnCI,
                "On the GitHub Actions Linux runner the old driver sends the login request to the invalid proxy despite this bypass list and retries until the connection timeout");
        }

        // TODO(SNOW-4382580): useProxy/proxyHost/proxyPort/nonProxyHosts connection-string options are ignored
        Skip.FutureMilestone();

        var account = Setting(BaseConnectionString(), "account");
        using var conn = CreateConnection(
            BaseConnectionString()
            + $";connection_timeout=30;useProxy=true;proxyHost=Invalid;proxyPort=8080;nonProxyHosts=*.foo.com %7C{account}.snowflakecomputing.com|*{ResolveHost()}*");

        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(ConnectionState.Open, conn.State);
    }

    [SnowflakeFact]
    public async Task ShouldAllowRepeatedCloseWithCancellationTokenAsync()
    {
        using var conn = CreateConnection(BaseConnectionString());
        Assert.Equal(ConnectionState.Closed, conn.State);

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Closed, conn.State);

        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn.State);

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Closed, conn.State);

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Closed, conn.State);
    }

    [SnowflakeFact]
    public async Task ShouldAllowRepeatedCloseAsync()
    {
        using var conn = CreateConnection(BaseConnectionString());
        Assert.Equal(ConnectionState.Closed, conn.State);

        await conn.CloseAsync(CancellationToken.None).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Closed, conn.State);

        await conn.OpenAsync(CancellationToken.None).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn.State);

        await conn.CloseAsync(CancellationToken.None).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Closed, conn.State);

        await conn.CloseAsync(CancellationToken.None).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Closed, conn.State);
    }

    [SnowflakeFact]
    public async Task ShouldTrackExplicitTransactionOperationsAsync()
    {
        // TODO(SNOW-4382589): BeginTransaction throws NotImplementedException
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var conn = CreateDbConnection(BaseConnectionString());
        await conn.OpenAsync(ct).ConfigureAwait(false);
        Assert.False(await HasActiveTransactionAsync(conn, ct).ConfigureAwait(false));

        var trans = await conn.BeginTransactionAsync(ct).ConfigureAwait(false);
        Assert.True(await HasActiveTransactionAsync(conn, ct).ConfigureAwait(false));
        await trans.RollbackAsync(ct).ConfigureAwait(false);
        Assert.False(await HasActiveTransactionAsync(conn, ct).ConfigureAwait(false));

        await (await conn.BeginTransactionAsync(ct).ConfigureAwait(false)).RollbackAsync(ct).ConfigureAwait(false);
        Assert.False(await HasActiveTransactionAsync(conn, ct).ConfigureAwait(false));

        await (await conn.BeginTransactionAsync(ct).ConfigureAwait(false)).CommitAsync(ct).ConfigureAwait(false);
        Assert.False(await HasActiveTransactionAsync(conn, ct).ConfigureAwait(false));
    }

    [SnowflakeFact]
    public async Task ShouldSetQueryTagFromConnectStringAsync()
    {
        const string expectedQueryTag = "Test QUERY_TAG 12345";
        using var conn = CreateConnection(BaseConnectionString() + $";query_tag={expectedQueryTag}");

        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        using var command = conn.CreateCommand();
        command.CommandText = "SELECT QUERY_TAG FROM table(information_schema.query_history_by_session())";
        var queryTag = await command.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(expectedQueryTag, queryTag);
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithMfaPasscodeAsSecureStringAsync()
    {
        Skip.When(IsRunningInJenkins(), "Test is skipped on Jenkins.");
        Skip.When(
            true,
            "Requires a user enrolled in Duo MFA, ALLOW_CLIENT_MFA_CACHING on the account and a passcode supplied manually at a breakpoint");

        using var conn = CreateConnection(BaseConnectionString() + ";minPoolSize=2;application=DuoTest;");
        conn.Passcode = ToSecureString("$(set proper passcode)");

        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(ConnectionState.Open, conn.State);
    }

    private string BaseConnectionString()
    {
        using var conn = Fixture.Factory.Create(Output);
        return conn.ConnectionString;
    }

    private string BaseConnectionStringWithoutAuth()
    {
        var builder = new DbConnectionStringBuilder { ConnectionString = BaseConnectionString() };
        foreach (var key in s_authenticationKeys)
            builder.Remove(key);
        return builder.ConnectionString;
    }

    private string ConnectionStringWithInvalidUserName() =>
        $"{BaseConnectionStringWithoutAuth()};user=unknown;password=unknown_password;";

    private string ResolveHost()
    {
        var baseConnectionString = BaseConnectionString();
        return TryGetSetting(baseConnectionString, "host", out var host)
            ? host
            : $"{Setting(baseConnectionString, "account")}.snowflakecomputing.com";
    }

    private ISnowflakeDbConnection CreateConnection(string connectionString) =>
        Fixture.Factory.Create(Output, connectionString);

    private static DbConnection CreateDbConnection(string connectionString)
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
            return new OldSnowflakeDbConnection(connectionString);
        return new NewSnowflakeDbConnection(connectionString);
    }

    private static Task CloseDbConnectionAsync(DbConnection connection, CancellationToken cancellationToken) =>
        connection is OldSnowflakeDbConnection old
            ? old.CloseAsync(cancellationToken)
            : ((NewSnowflakeDbConnection)connection).CloseAsync(cancellationToken);

    private void DropTable(string tableName)
    {
        using var conn = CreateConnection(BaseConnectionString());
        conn.Open();
        using var cmd = conn.CreateCommand();
        cmd.CommandText = $"DROP TABLE IF EXISTS {tableName}";
        cmd.ExecuteNonQuery();
    }

    private static string Setting(string connectionString, string key)
    {
        Assert.True(TryGetSetting(connectionString, key, out var value), $"Connection string has no '{key}'");
        return value;
    }

    private static bool TryGetSetting(string connectionString, string key, out string value)
    {
        var builder = new DbConnectionStringBuilder { ConnectionString = connectionString };
        if (builder.TryGetValue(key, out var raw) && raw is string text)
        {
            value = text;
            return true;
        }

        value = string.Empty;
        return false;
    }

    private static bool IsRunningInJenkins() =>
        !string.IsNullOrEmpty(Environment.GetEnvironmentVariable("JENKINS_URL")) ||
        !string.IsNullOrEmpty(Environment.GetEnvironmentVariable("BUILD_NUMBER")) ||
        !string.IsNullOrEmpty(Environment.GetEnvironmentVariable("JOB_NAME"));

    private static SecureString ToSecureString(string text)
    {
        var secure = new SecureString();
        foreach (var c in text)
            secure.AppendChar(c);
        return secure;
    }

    private static DbException AssertSnowflakeDbException(Exception exception)
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
            return Assert.IsType<OldSnowflakeDbException>(exception);
        return Assert.IsType<NewSnowflakeDbException>(exception);
    }

    private static void AssertIsConnectionFailure(Exception exception)
    {
        var snowflakeException = AssertSnowflakeDbException(exception);
        var sqlState = snowflakeException is OldSnowflakeDbException old
            ? old.SqlState
            : ((NewSnowflakeDbException)snowflakeException).SqlState;
        Assert.Equal(ConnectionFailureSqlState, sqlState);
    }

    private static void AssertConnectionIsNotOpen(ISnowflakeDbConnection? connection)
    {
        Assert.NotNull(connection);
        Assert.False(connection.IsOpen());
        Assert.Equal(ConnectionState.Closed, connection.State);
    }

    private static async Task<bool> HasActiveTransactionAsync(DbConnection connection, CancellationToken cancellationToken)
    {
        using var command = connection.CreateCommand();
        command.CommandText = "SELECT CURRENT_TRANSACTION()";
        var transactionId = await command.ExecuteScalarAsync(cancellationToken).ConfigureAwait(false);
        return transactionId is not null and not DBNull;
    }
}
