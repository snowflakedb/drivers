using System.Collections.Generic;
using System.Data.Common;
using System.Diagnostics;
using System.Globalization;
using System.Net;
using System.Net.Http;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Security;
using System.Text.Json;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.Reference.Fixtures;
using Snowflake.Data.Tests.Reference.Wiremock;
using Snowflake.Data.Tests.Utilities;
using Snowflake.Data.Tests.Wiremock;
using NewSnowflakeDbConnection = Snowflake.Data.SnowflakeDbConnection;
using NewSnowflakeDbException = Snowflake.Data.SnowflakeDbException;
using OldSnowflakeDbConnection = Snowflake.Data.Client.SnowflakeDbConnection;
using OldSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.Reference;

public sealed class SFConnectionAsyncTest : ReferenceTestBase
{
    private const string ConnectionFailureSqlState = "08006";
    private const int InternalErrorCode = 270001;
    private const int UnknownAuthenticatorErrorCode = 270011;
    private const int ObjectNonexistentOrNotAuthorizedErrorCode = 390201;
    private const int ExtOAuthAccessTokenInvalidErrorCode = 390303;
    private const int OldDriverDefaultRetryTimeoutSeconds = 300;
    private const int QueryCancelledErrorCode = 270005;
    private const string DelayedLoginUser = "wiremock_login_delayed";
    private const string UnavailableLoginUser = "wiremock_login_503_always";
    private const string OldDriverOktaRetryCountersAreMocked =
        "The old driver counts Okta retries from response headers that only its test-only MockOktaRestRequester sets; against an HTTP server both counters stay at zero";

    private static readonly string[] s_authenticationKeys =
        ["user", "password", "token", "authenticator", "private_key_file", "private_key", "private_key_pwd"];

    public SFConnectionAsyncTest(ReferenceITFixture fixture, ITestOutputHelper output)
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
    public async Task ShouldSendApplicationPathDuringAuthenticationAsync()
    {
        Skip.For(SkipCondition.RunOnlyOnCI, "Test runs only on CI.");
        Skip.OnNewDriver("BD#application-path-not-sent", "The .NET wrapper identity does not set application_path");

        var applicationPath = await ReadLoginApplicationPathAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        var lowerPath = applicationPath.ToLowerInvariant();
        Assert.True(
            lowerPath.Contains("snowflake.data.tests")
            && lowerPath.Contains("bin")
            && (lowerPath.EndsWith(".dll", StringComparison.Ordinal) || lowerPath.EndsWith(".exe", StringComparison.Ordinal)),
            $"APPLICATION_PATH should contain 'snowflake.data.tests' and 'bin' and end with .dll or .exe. Got: {applicationPath}");
    }

    [SnowflakeFact]
    public async Task ShouldSendUnknownApplicationPathDuringAuthenticationAsync()
    {
        Skip.OnOldDriver("BD#application-path-not-sent", "The old driver sends the path of the running executable");

        var applicationPath = await ReadLoginApplicationPathAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal("UNKNOWN", applicationPath);
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
    public async Task ShouldFailLoginWhenMaxRetryReachedAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await ReferenceWiremock.StartWithMappingsAsync(ct, "auth/login_503_always.json").ConfigureAwait(false);
        using var conn = CreateConnection(
            ReferenceWiremock.BuildHttpConnectionString(wiremock, $"user={UnavailableLoginUser}", MaxHttpRetriesSetting(7)));

        Assert.Equal(ConnectionState.Closed, conn.State);
        var stopwatch = Stopwatch.StartNew();
        var exception = await Assert.ThrowsAnyAsync<DbException>(() => conn.OpenAsync(ct)).ConfigureAwait(false);
        stopwatch.Stop();

        AssertSnowflakeDbException(exception);
        // Seven retries start with a 1 s backoff that can randomly drop to 0, so the total backoff
        // ranges from 1 + 0 + 0 + 0 + 0 + 0 + 0 = 1 s to 1 + 2 + 5 + 10 + 21 + 42 + 85 = 166 s.
        Assert.InRange(stopwatch.ElapsedMilliseconds, 1 * 1000, 166 * 1000);
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
    public async Task ShouldRejectInvalidConnectionStringAsync()
    {
        Skip.When(true, "TODO Bug: SNOW-3552882");

        string[] invalidStrings =
        [
            "ACCOUNT=testaccount;user=testuser",
            "ACCOUNT=A=C;USER=testuser;password=123;key",
            "complete_invalid_string",
        ];
        int[] expectedErrorCode = [270006, 270008, 270008];

        using var conn = CreateConnection(string.Empty);
        for (var i = 0; i < invalidStrings.Length; i++)
        {
            conn.ConnectionString = invalidStrings[i];
            var exception = await Assert.ThrowsAnyAsync<DbException>(
                () => conn.OpenAsync(TestContext.Current.CancellationToken)).ConfigureAwait(false);

            Assert.Equal(expectedErrorCode[i], AssertSnowflakeDbException(exception).ErrorCode);
        }
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
    public async Task ShouldRejectUnknownAuthenticatorAsync()
    {
        // TODO(SNOW-4382591): unknown authenticators surface as ServiceException, not SnowflakeDbException
        Skip.FutureMilestone();

        string[] wrongAuthenticators =
        [
            "http://snowflakecomputing.okta.com",
            "https://snowflake.com",
            "unknown",
        ];

        foreach (var wrongAuthenticator in wrongAuthenticators)
        {
            using var conn = CreateConnection(
                "scheme=http;host=snowflake.com;port=8080;user=test;password=test;account=test;authenticator=" + wrongAuthenticator);
            var exception = await Assert.ThrowsAnyAsync<DbException>(
                () => conn.OpenAsync(TestContext.Current.CancellationToken)).ConfigureAwait(false);

            Assert.Equal(UnknownAuthenticatorErrorCode, AssertSnowflakeDbException(exception).ErrorCode);
        }
    }

    [SnowflakeFact]
    public async Task ShouldRetryOktaConnectionUntilMaxTimeoutAsync()
    {
        Skip.When(TestDriver.Current == TestDriver.Kind.Old, OldDriverOktaRetryCountersAreMocked);
        // TODO(SNOW-4382591): SnowflakeDbException
        Skip.FutureMilestone();

        var expectedMaxRetryCount = 15;
        var exception = await OpenWithOktaSamlMissingPostbackAsync(
            expectedMaxRetryCount, 450, TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(InternalErrorCode, AssertSnowflakeDbException(exception).ErrorCode);
        var messages = string.Join("\n", CollectExceptions(exception).Select(e => e.Message));
        Assert.Contains($"The retry count has reached its limit of {expectedMaxRetryCount}", messages);
        Assert.Contains("while trying to authenticate through Okta", messages);
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
    public async Task ShouldCancelLoginBeforeTimeoutAsync()
    {
        // TODO(SNOW-4382580): a cancelled OpenAsync throws ServiceException instead of TaskCanceledException and leaves State as Connecting
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await ReferenceWiremock.StartWithMappingsAsync(ct, "auth/login_delayed.json").ConfigureAwait(false);
        var timeoutSec = 0;
        using var conn = CreateConnection(
            ReferenceWiremock.BuildHttpConnectionString(
                wiremock, $"user={DelayedLoginUser}", $"{LoginTimeoutKey()}={timeoutSec}", MaxHttpRetriesSetting(0)));

        Assert.Equal(ConnectionState.Closed, conn.State);

        using var connectionCancelToken = CancellationTokenSource.CreateLinkedTokenSource(ct);
        var openTask = conn.OpenAsync(connectionCancelToken.Token);

        Assert.Equal(ConnectionState.Connecting, conn.State);

        connectionCancelToken.Cancel();
        await Assert.ThrowsAsync<TaskCanceledException>(() => openTask).ConfigureAwait(false);

        Assert.Equal(ConnectionState.Closed, conn.State);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.Thrice)]
    public async Task ShouldTimeOutLoginAfterConnectionTimeoutAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException
        // TODO(SNOW-4382580): a failed OpenAsync leaves State as Connecting and ConnectionTimeout does not report the login timeout
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await ReferenceWiremock.StartWithMappingsAsync(
            ct, "auth/login_delayed.json", "auth/login_success_any.json").ConfigureAwait(false);
        var timeoutSec = 5;
        var timeoutSetting = $"{LoginTimeoutKey()}={timeoutSec}";
        using var conn = CreateConnection(
            ReferenceWiremock.BuildHttpConnectionString(wiremock, $"user={DelayedLoginUser}", timeoutSetting, MaxHttpRetriesSetting(0)));

        Assert.Equal(ConnectionState.Closed, conn.State);

        var stopwatch = Stopwatch.StartNew();
        var exception = await Assert.ThrowsAnyAsync<DbException>(() => conn.OpenAsync(ct)).ConfigureAwait(false);
        stopwatch.Stop();

        AssertHasErrorCodeInExceptionChain(exception, InternalErrorCode);
        // The retry count is infinite, so the login times out after the connection timeout.
        Assert.InRange(stopwatch.ElapsedMilliseconds, timeoutSec * 1000 - 50, long.MaxValue);
        Assert.Equal(ConnectionState.Closed, conn.State);

        using var openedConn = CreateConnection(
            ReferenceWiremock.BuildHttpConnectionString(wiremock, timeoutSetting, MaxHttpRetriesSetting(0)));
        await openedConn.OpenAsync(ct).ConfigureAwait(false);
        Assert.Equal(timeoutSec, openedConn.ConnectionTimeout);
    }

    [SnowflakeFact]
    public async Task ShouldReturnCancelWithinCleanupWaitWhenPostHangsAsync()
    {
        Skip.OnNewDriver("BD#cancel-cleanup-wait-and-abort-timeout-not-configurable", "cleanup_wait has no effect");

        const int CleanupWaitSeconds = 2;
        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await StartWiremockWithHangingAbortAsync(ct).ConfigureAwait(false);

        var elapsed = await CancelHangingQueryAsync(wiremock, $"cleanup_wait={CleanupWaitSeconds}", ct).ConfigureAwait(false);

        Assert.Contains(await ReferenceWiremock.GetRecordedRequestsAsync(wiremock, ct).ConfigureAwait(false), IsAbortRequest);
        // The bound allows 1 s for the external token, 2 s of cleanup_wait and a generous tolerance.
        Assert.True(
            elapsed.TotalSeconds < CleanupWaitSeconds + 30,
            $"Cancel() should return within cleanup_wait ({CleanupWaitSeconds}s) but took {elapsed.TotalSeconds:F1}s");
    }

    [SnowflakeFact]
    public async Task ShouldReturnCancelImmediatelyWhenCleanupWaitIsZeroAsync()
    {
        Skip.OnNewDriver("BD#cancel-cleanup-wait-and-abort-timeout-not-configurable", "cleanup_wait has no effect");

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await StartWiremockWithHangingAbortAsync(ct).ConfigureAwait(false);

        var elapsed = await CancelHangingQueryAsync(wiremock, "cleanup_wait=0", ct).ConfigureAwait(false);

        Assert.True(
            elapsed.TotalSeconds < 5,
            $"Cancel() with cleanup_wait=0 should return immediately but took {elapsed.TotalSeconds:F1}s");
    }

    [SnowflakeFact]
    public async Task ShouldApplyAbortRequestTimeoutToCancelRequestAsync()
    {
        Skip.OnNewDriver("BD#cancel-cleanup-wait-and-abort-timeout-not-configurable", "abort_request_timeout has no effect");

        const int AbortTimeoutSeconds = 3;
        const int CleanupWaitSeconds = 5;
        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await StartWiremockWithHangingAbortAsync(ct).ConfigureAwait(false);

        var elapsed = await CancelHangingQueryAsync(
            wiremock, $"abort_request_timeout={AbortTimeoutSeconds};cleanup_wait={CleanupWaitSeconds}", ct).ConfigureAwait(false);

        Assert.Contains(await ReferenceWiremock.GetRecordedRequestsAsync(wiremock, ct).ConfigureAwait(false), IsAbortRequest);
        // The token fires after 1 s and the abort request hangs, so the cancel returns when abort_request_timeout
        // expires (about 1 + AbortTimeoutSeconds); cleanup_wait alone would return after 1 + CleanupWaitSeconds.
        Assert.InRange(elapsed.TotalSeconds, AbortTimeoutSeconds, CleanupWaitSeconds);
    }

    [SnowflakeFact]
    public async Task ShouldReturnCancelAfterFixedCleanupWaitWhenAbortHangsAsync()
    {
        Skip.OnOldDriver("BD#cancel-cleanup-wait-and-abort-timeout-not-configurable", "The old driver honors cleanup_wait");
        // TODO(SNOW-4382591): SnowflakeDbException constructors throw NotImplementedException
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await StartWiremockWithHangingAbortAsync(ct).ConfigureAwait(false);

        var elapsed = await CancelHangingQueryAsync(wiremock, "cleanup_wait=0", ct).ConfigureAwait(false);

        Assert.InRange(elapsed.TotalSeconds, 1 + 5, 1 + 30);
    }

    [SnowflakeFact]
    public async Task ShouldFailFastForNonRetried404OnLoginAsync()
    {
        // TODO(SNOW-4382591): HTTP login failures surface as ServiceException, not SnowflakeDbException
        Skip.FutureMilestone();

        using var conn = CreateConnection(
            $"host=docs.snowflake.com/404;{LoginTimeoutKey()}=0;account=testFailFast;user=testFailFast;password=testFailFast;certRevocationCheckMode=enabled;");
        Assert.Equal(ConnectionState.Closed, conn.State);

        var exception = await Assert.ThrowsAnyAsync<DbException>(
            () => conn.OpenAsync(TestContext.Current.CancellationToken)).ConfigureAwait(false);

        AssertSnowflakeDbException(exception);
        var chain = CollectExceptions(exception);
        Assert.Contains(chain, e => e switch
        {
            DbException se when IsDriverSnowflakeDbException(se) => se.ErrorCode == (int)HttpStatusCode.NotFound,
            HttpRequestException he => he.Message.Contains(((int)HttpStatusCode.NotFound).ToString(CultureInfo.InvariantCulture)),
            _ => false,
        });
        Assert.Contains(chain, e => e.Message.Contains("404 (Not Found)"));
        Assert.Equal(ConnectionState.Closed, conn.State);
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
    public async Task ShouldBreakConnectionWhenCloseFailsAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException
        // TODO(SNOW-4382580): a failed Close leaves State as Open instead of Broken
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await ReferenceWiremock.StartWithMappingsAsync(
            ct, "auth/login_success_any.json", "session/logout_400_always.json").ConfigureAwait(false);
        using var conn = CreateConnection(ReferenceWiremock.BuildHttpConnectionString(wiremock, "poolingEnabled=false"));
        Assert.Equal(ConnectionState.Closed, conn.State);

        await conn.OpenAsync(ct).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn.State);

        var exception = await Assert.ThrowsAnyAsync<Exception>(() => conn.CloseAsync(ct)).ConfigureAwait(false);
        AssertIsLogoutFailure(exception);
        Assert.Equal(ConnectionState.Broken, conn.State);
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
    public async Task ShouldReportOktaRetryLimitInExceptionChainAsync()
    {
        Skip.When(TestDriver.Current == TestDriver.Kind.Old, OldDriverOktaRetryCountersAreMocked);
        // TODO(SNOW-4382591): SnowflakeDbException
        Skip.FutureMilestone();

        var expectedMaxRetryCount = 15;
        var exception = await OpenWithOktaSamlMissingPostbackAsync(
            expectedMaxRetryCount, 450, TestContext.Current.CancellationToken).ConfigureAwait(false);

        AssertHasErrorCodeInExceptionChain(exception, InternalErrorCode);
        var chain = CollectExceptions(exception);
        Assert.Contains(chain, e => e.Message.Contains($"The retry count has reached its limit of {expectedMaxRetryCount}"));
        Assert.Contains(chain, e => e.Message.Contains("while trying to authenticate through Okta"));
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

    [SnowflakeTheory(RetriesCount = RetriesCount.Thrice)]
    [InlineData("connection_timeout=5;", false)]
    [InlineData("", true)]
    public async Task ShouldThrowWhenOpenAsyncConnectsToUnreachableHostAsync(string extraParameters, bool expectCancellation)
    {
        // TODO(SNOW-4382580): proxy options are ignored and a failed OpenAsync leaves State as Connecting
        Skip.FutureMilestone();

        using var connection = CreateConnection(
            "account=testAccount;user=testUser;password=testPassword;useProxy=true;proxyHost=no.such.pro.xy;proxyPort=8080;certRevocationCheckMode=enabled;"
            + extraParameters);

        if (expectCancellation)
        {
            await Assert.ThrowsAsync<TaskCanceledException>(() => connection.OpenAsync(CancellationToken.None)).ConfigureAwait(false);
        }
        else
        {
            var thrown = await Assert.ThrowsAnyAsync<DbException>(() => connection.OpenAsync(CancellationToken.None)).ConfigureAwait(false);

            Assert.Equal(InternalErrorCode, AssertSnowflakeDbException(thrown).ErrorCode);
            AssertIsConnectionFailure(thrown);
        }

        Assert.Equal(ConnectionState.Closed, connection.State);
    }

    [SnowflakeFact]
    public async Task ShouldThrowWhenOpenAsyncIsCancelledAsync()
    {
        // TODO(SNOW-4382580): proxy options are ignored, so the login fails before the cancellation fires
        Skip.FutureMilestone();

        using var connection = CreateConnection(
            "account=testAccount;user=testUser;password=testPassword;useProxy=true;proxyHost=no.such.pro.xy;proxyPort=8080;certRevocationCheckMode=enabled;");
        using var shortCancellation = new CancellationTokenSource(TimeSpan.FromSeconds(5));

        await Assert.ThrowsAsync<TaskCanceledException>(() => connection.OpenAsync(shortCancellation.Token)).ConfigureAwait(false);

        Assert.Equal(ConnectionState.Closed, connection.State);
    }

    [SnowflakeFact]
    public async Task ShouldNotBlockOnHangingCloseAsync()
    {
        Skip.OnNewDriver("BD#session-not-closed-on-gc", "A finalized connection does not send a logout request");

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await ReferenceWiremock.StartWithMappingsAsync(
            ct, "auth/login_success_any.json", "session/logout_delayed.json").ConfigureAwait(false);
        OpenAndAbandonConnection(ReferenceWiremock.BuildHttpConnectionString(wiremock, "poolingEnabled=false"));

        var watchClose = Stopwatch.StartNew();
        GC.Collect();
        GC.WaitForPendingFinalizers();
        watchClose.Stop();
        var logoutRequests = await WaitForLogoutRequestsAsync(wiremock, TimeSpan.FromSeconds(15), ct).ConfigureAwait(false);

        Assert.Single(logoutRequests);
        Assert.True(watchClose.Elapsed.Duration() < TimeSpan.FromSeconds(5));
    }

    [SnowflakeFact]
    public async Task ShouldStayOpenWhenCloseIsCancelledAsync()
    {
        // TODO(SNOW-4382580): CloseAsync does not observe cancellation once the logout request is in flight
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await ReferenceWiremock.StartWithMappingsAsync(
            ct, "auth/login_success_any.json", "session/logout_delayed.json").ConfigureAwait(false);
        using var conn = CreateConnection(ReferenceWiremock.BuildHttpConnectionString(wiremock, "poolingEnabled=false"));
        await conn.OpenAsync(ct).ConfigureAwait(false);
        using var closeCancellation = CancellationTokenSource.CreateLinkedTokenSource(ct);
        closeCancellation.CancelAfter(TimeSpan.FromSeconds(1));

        await Assert.ThrowsAsync<TaskCanceledException>(() => conn.CloseAsync(closeCancellation.Token)).ConfigureAwait(false);

        Assert.Equal(ConnectionState.Open, conn.State);
    }

    [SnowflakeFact]
    public async Task ShouldCancelQueryQuicklyWithDefaultCleanupSettingsAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException constructors throw NotImplementedException
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await ReferenceWiremock.StartWithMappingsAsync(
            ct,
            "auth/login_success_any.json",
            "query/query_hanging.json",
            "query/abort_request_success.json",
            "session/logout_success.json").ConfigureAwait(false);
        using var conn = CreateConnection(ReferenceWiremock.BuildHttpConnectionString(wiremock, "poolingEnabled=false"));

        await conn.OpenAsync(ct).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn.State);

        using var cts = CancellationTokenSource.CreateLinkedTokenSource(ct);
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT SLOW_QUERY()";
        cmd.CommandTimeout = 0;

        cts.CancelAfter(TimeSpan.FromSeconds(2));

        var stopwatch = Stopwatch.StartNew();
        var thrown = await Assert.ThrowsAsync<OperationCanceledException>(
            () => cmd.ExecuteScalarAsync(cts.Token)).ConfigureAwait(false);

        Assert.Equal(QueryCancelledErrorCode, AssertSnowflakeDbException(thrown.InnerException!).ErrorCode);
        Assert.True(
            stopwatch.Elapsed.TotalSeconds < 30,
            $"Default cancel should complete quickly. Elapsed: {stopwatch.Elapsed.TotalSeconds:F2}s");
    }

    [SnowflakeFact(RetriesCount = RetriesCount.Thrice)]
    public async Task ShouldCloseSessionWhenGarbageCollectorFinalizesConnectionAsync()
    {
        Skip.OnNewDriver("BD#session-not-closed-on-gc", "A finalized connection does not send a logout request");

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await ReferenceWiremock.StartWithMappingsAsync(
            ct, "auth/login_success_any.json", "session/logout_success.json").ConfigureAwait(false);
        OpenAndAbandonConnection(
            ReferenceWiremock.BuildHttpConnectionString(
                wiremock, "poolingEnabled=false", "application=TestGarbageCollectorCloseSession"));

        GC.Collect();
        var logoutRequests = await WaitForLogoutRequestsAsync(wiremock, TimeSpan.FromSeconds(30), ct).ConfigureAwait(false);

        Assert.Single(logoutRequests);
    }

    [SnowflakeFact]
    public async Task ShouldNotCloseSessionWhenGarbageCollectorCollectsConnectionAsync()
    {
        Skip.OnOldDriver("BD#session-not-closed-on-gc", "The old driver's finalizer sends a logout request");

        var ct = TestContext.Current.CancellationToken;
        using var wiremock = await ReferenceWiremock.StartWithMappingsAsync(
            ct, "auth/login_success_any.json", "session/logout_success.json").ConfigureAwait(false);
        OpenAndAbandonConnection(
            ReferenceWiremock.BuildHttpConnectionString(
                wiremock, "poolingEnabled=false", "application=TestGarbageCollectorCloseSession"));

        GC.Collect();
        GC.WaitForPendingFinalizers();
        await Task.Delay(TimeSpan.FromSeconds(2), ct).ConfigureAwait(false);

        Assert.DoesNotContain(
            await ReferenceWiremock.GetRecordedRequestsAsync(wiremock, ct).ConfigureAwait(false),
            IsLogoutRequest);
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

    private static bool IsDriverSnowflakeDbException(Exception exception) =>
        TestDriver.Current == TestDriver.Kind.Old
            ? exception is OldSnowflakeDbException
            : exception is NewSnowflakeDbException;

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

    private static string LoginTimeoutKey() =>
        BehaviorDifference.OldDriverOnly("BD#connection-timeout-key-not-recognized")
            ? "connection_timeout"
            : "login_timeout";

    private static string MaxHttpRetriesSetting(int retries) =>
        BehaviorDifference.OldDriverOnly("BD#max-http-retries-key-not-recognized")
            ? $"maxHttpRetries={retries}"
            : $"retry_max_attempts={retries + 1}";

    private static void AssertHasErrorCodeInExceptionChain(Exception exception, int expectedErrorCode) =>
        Assert.Contains(
            CollectExceptions(exception),
            e => e is DbException se && IsDriverSnowflakeDbException(se) && se.ErrorCode == expectedErrorCode);

    private static void AssertIsLogoutFailure(Exception exception)
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
            Assert.IsType<HttpRequestException>(exception);
        else
            Assert.IsType<NewSnowflakeDbException>(exception);
    }

    private static bool IsAbortRequest(RecordedRequest request) =>
        request.Method == "POST" && request.Url.StartsWith("/queries/v1/abort-request", StringComparison.Ordinal);

    private static bool IsLogoutRequest(RecordedRequest request) =>
        request.Method == "POST" && request.Url.StartsWith("/session?delete=true", StringComparison.Ordinal);

    private static async Task<IReadOnlyList<RecordedRequest>> WaitForLogoutRequestsAsync(
        WiremockClient wiremock,
        TimeSpan timeout,
        CancellationToken cancellationToken)
    {
        var stopwatch = Stopwatch.StartNew();
        while (true)
        {
            var logoutRequests = (await ReferenceWiremock.GetRecordedRequestsAsync(wiremock, cancellationToken).ConfigureAwait(false))
                .Where(IsLogoutRequest)
                .ToList();
            if (logoutRequests.Count > 0 || stopwatch.Elapsed > timeout)
                return logoutRequests;
            await Task.Delay(TimeSpan.FromMilliseconds(200), cancellationToken).ConfigureAwait(false);
        }
    }

    private static async Task<bool> HasActiveTransactionAsync(DbConnection connection, CancellationToken cancellationToken)
    {
        using var command = connection.CreateCommand();
        command.CommandText = "SELECT CURRENT_TRANSACTION()";
        var transactionId = await command.ExecuteScalarAsync(cancellationToken).ConfigureAwait(false);
        return transactionId is not null and not DBNull;
    }

    private static Task<WiremockClient> StartWiremockWithHangingAbortAsync(CancellationToken cancellationToken) =>
        ReferenceWiremock.StartWithMappingsAsync(
            cancellationToken,
            "auth/login_success_any.json",
            "query/query_hanging.json",
            "query/abort_request_hanging.json",
            "session/logout_success.json");

    private async Task<TimeSpan> CancelHangingQueryAsync(
        WiremockClient wiremock,
        string cancelSettings,
        CancellationToken cancellationToken)
    {
        using var conn = CreateConnection(ReferenceWiremock.BuildHttpConnectionString(wiremock, "poolingEnabled=false", cancelSettings));
        await conn.OpenAsync(cancellationToken).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn.State);

        using var cts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT LONG_RUNNING_QUERY()";
        cmd.CommandTimeout = 0;

        cts.CancelAfter(TimeSpan.FromSeconds(1));

        var stopwatch = Stopwatch.StartNew();
        await Assert.ThrowsAsync<OperationCanceledException>(() => cmd.ExecuteScalarAsync(cts.Token)).ConfigureAwait(false);
        stopwatch.Stop();
        return stopwatch.Elapsed;
    }

    private async Task<string> ReadLoginApplicationPathAsync(CancellationToken cancellationToken)
    {
        using var wiremock = await ReferenceWiremock.StartWithMappingsAsync(
            cancellationToken, "auth/login_success_any.json", "session/logout_success.json").ConfigureAwait(false);
        using (var conn = CreateConnection(ReferenceWiremock.BuildHttpConnectionString(wiremock, "poolingEnabled=false")))
        {
            await conn.OpenAsync(cancellationToken).ConfigureAwait(false);
            await conn.CloseAsync(cancellationToken).ConfigureAwait(false);
        }

        var requests = await ReferenceWiremock.GetRecordedRequestsAsync(wiremock, cancellationToken).ConfigureAwait(false);
        var login = Assert.Single(requests, r => r.Url.StartsWith("/session/v1/login-request", StringComparison.Ordinal));
        using var body = JsonDocument.Parse(login.Body);
        return body.RootElement.GetProperty("data").GetProperty("CLIENT_ENVIRONMENT").GetProperty("APPLICATION_PATH").GetString()!;
    }

    private async Task<Exception> OpenWithOktaSamlMissingPostbackAsync(
        int maxRetryCount,
        int maxRetryTimeoutSeconds,
        CancellationToken cancellationToken)
    {
        using var wiremock = await ReferenceWiremock.StartHttpsWithMappingsAsync(
            cancellationToken, "auth/okta_token_cookie_token.json", "auth/okta_sso_missing_form_action.json").ConfigureAwait(false);
        var oktaBase = $"https://localhost:{wiremock.HttpsPort}";
        await wiremock.AddMappingAsync(
            "auth/authenticator_request_okta.json",
            placeholders: new Dictionary<string, string> { ["{{OKTA_BASE_URL}}"] = oktaBase },
            cancellationToken: cancellationToken).ConfigureAwait(false);
        using var conn = CreateConnection(
            ReferenceWiremock.BuildHttpConnectionString(
                wiremock,
                $"authenticator={oktaBase}/okta",
                "user=test",
                "password=test",
                $"custom_root_store_path={wiremock.WiremockCaPemPath}",
                $"retry_max_attempts={maxRetryCount}",
                $"login_timeout={maxRetryTimeoutSeconds}"));

        return await Assert.ThrowsAnyAsync<Exception>(() => conn.OpenAsync(cancellationToken)).ConfigureAwait(false);
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private void OpenAndAbandonConnection(string connectionString)
    {
        var conn = CreateConnection(connectionString);
        conn.Open();
        Assert.Equal(ConnectionState.Open, conn.State);
    }

    private static List<Exception> CollectExceptions(Exception? exception)
    {
        var collected = new List<Exception>();
        switch (exception)
        {
            case null:
                break;
            case AggregateException aggregate:
                var inner = aggregate.Flatten().InnerExceptions;
                collected.AddRange(inner);
                foreach (var e in inner)
                    collected.AddRange(CollectExceptions(e.InnerException));
                break;
            default:
                collected.AddRange(CollectExceptions(exception.InnerException));
                collected.Add(exception);
                break;
        }

        return collected;
    }
}
