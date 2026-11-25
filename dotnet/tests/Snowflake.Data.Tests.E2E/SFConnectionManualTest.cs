extern alias OldDriver;
using System.Data.Common;
using System.Diagnostics;
using System.Text;
using System.Text.RegularExpressions;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E.Fixtures;
using Snowflake.Data.Tests.Utilities;
using NewSnowflakeDbConnection = Snowflake.Data.Client.SnowflakeDbConnection;
using NewSnowflakeDbConnectionPool = Snowflake.Data.Client.SnowflakeDbConnectionPool;
using NewSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;
using OldSnowflakeDbConnection = OldDriver::Snowflake.Data.Client.SnowflakeDbConnection;
using OldSnowflakeDbConnectionPool = OldDriver::Snowflake.Data.Client.SnowflakeDbConnectionPool;
using OldSnowflakeDbException = OldDriver::Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.E2E;

[Collection(nameof(SFConnectionManualTest))]
public sealed class SFConnectionManualTest : LiveE2ETestBase
{
    private const string ManualInteraction = "This test requires manual interaction and therefore cannot be run in CI";
    private const string ManualSetup = "This test requires manual setup and therefore cannot be run in CI";
    private const string NotConfiguredForCi =
        "Ignore this test until configuration is setup for CI integration. Can be run manually.";
    private const string ManualOnly = "Manual test only";
    private const string ManualParserBreakpoint =
        "Ignore this test, please test this manual with breakpoint at SFSessionProperty::ParseConnectionString() to verify";
    private const string FourHours = "Ignore this test. Please run this manually, since it takes 4 hrs to finish.";
    private const int OldDriverDefaultRetryTimeoutSeconds = 300;
    private const int InternalErrorCode = 270001;
    private const string ConnectionFailureSqlState = "08006";

    private static readonly string[] s_authenticationKeys =
        ["user", "password", "token", "authenticator", "private_key_file", "private_key", "private_key_pwd"];

    public SFConnectionManualTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithNativeOktaAsync()
    {
        Skip.When(true, "This test requires established dev Okta SSO and credentials matching Snowflake user");

        const string oktaUrl = "https://***.okta.com/";
        const string oktaUser = "***";
        const string oktaPassword = "***";
        using var conn = CreateConnection(
            BaseConnectionStringWithoutAuth() + $";authenticator={oktaUrl};user={oktaUser};password={oktaPassword};");

        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(ConnectionState.Open, conn.State);
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithOktaAsync()
    {
        Skip.When(true, ManualSetup);

        using var conn = CreateConnection(OktaConnectionString());

        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(ConnectionState.Open, conn.State);
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithOktaTwiceFollowingEachOtherAsync()
    {
        Skip.When(true, ManualSetup);

        using (var conn = CreateConnection(OktaConnectionString()))
        {
            await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.Equal(ConnectionState.Open, conn.State);
        }

        using (var conn = CreateConnection(OktaConnectionString()))
        {
            await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.Equal(ConnectionState.Open, conn.State);
        }
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithSsoUserDirectlyAsync()
    {
        Skip.When(true, ManualInteraction);

        using var conn = CreateConnection(
            BaseConnectionStringWithoutAuth() + ";authenticator=externalbrowser;user=qa@snowflakecomputing.com");
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn.State);

        Assert.False(GetPooling(conn.ConnectionString));
        using var command = conn.CreateCommand();
        command.CommandText = "SELECT CURRENT_USER()";
        Assert.Equal("QA", (await command.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))!.ToString());
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithSsoWithPoolingEnabledAsync()
    {
        Skip.When(true, ManualInteraction);

        using var conn = CreateConnection(
            BaseConnectionStringWithoutAuth() + ";authenticator=externalbrowser;user=qa@snowflakecomputing.com;POOLINGENABLED=TRUE");
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn.State);

        Assert.True(GetPooling(conn.ConnectionString));
        using var command = conn.CreateCommand();
        command.CommandText = "SELECT CURRENT_USER()";
        Assert.Equal("QA", (await command.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))!.ToString());
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithSsoUserAsync()
    {
        Skip.When(true, ManualInteraction);

        using var conn = CreateConnection(
            BaseConnectionStringWithoutAuth() + ";authenticator=externalbrowser;user=qa@snowflakecomputing.com");

        var connectTask = conn.OpenAsync(CancellationToken.None);
        await connectTask.ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn.State);
        using var command = conn.CreateCommand();
        command.CommandText = "SELECT CURRENT_USER()";
        var task = command.ExecuteScalarAsync(CancellationToken.None);
        Assert.Equal("QA", await task.ConfigureAwait(false));
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithSsoUserAndDisableConsoleLoginAsync()
    {
        Skip.When(true, ManualInteraction);

        using var conn = CreateConnection(
            BaseConnectionStringWithoutAuth()
            + ";authenticator=externalbrowser;user=qa@snowflakecomputing.com;disable_console_login=false;");
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn.State);

        using var command = conn.CreateCommand();
        command.CommandText = "SELECT CURRENT_USER()";
        Assert.Equal("QA", (await command.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))!.ToString());
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithSsoUserThroughOpenTaskAndDisableConsoleLoginAsync()
    {
        Skip.When(true, ManualInteraction);

        using var conn = CreateConnection(
            BaseConnectionStringWithoutAuth()
            + ";authenticator=externalbrowser;user=qa@snowflakecomputing.com;disable_console_login=false;");

        var connectTask = conn.OpenAsync(CancellationToken.None);
        await connectTask.ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn.State);
        using var command = conn.CreateCommand();
        command.CommandText = "SELECT CURRENT_USER()";
        var task = command.ExecuteScalarAsync(CancellationToken.None);
        Assert.Equal("QA", await task.ConfigureAwait(false));
    }

    [SnowflakeFact]
    public async Task ShouldTimeOutSsoConnectionAfter10sAsync()
    {
        Skip.When(true, ManualInteraction);

        const int waitSeconds = 10;
        var stopwatch = Stopwatch.StartNew();
        var exception = await Assert.ThrowsAnyAsync<DbException>(async () =>
        {
            using var conn = CreateConnection(
                BaseConnectionStringWithoutAuth()
                + $";authenticator=externalbrowser;user=qa@snowflakecomputing.com;BROWSER_RESPONSE_TIMEOUT={waitSeconds}");
            await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.Equal(ConnectionState.Open, conn.State);
            using var command = conn.CreateCommand();
            command.CommandText = "SELECT CURRENT_USER()";
            Assert.Equal("QA", (await command.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))!.ToString());
        }).ConfigureAwait(false);
        stopwatch.Stop();

        AssertSnowflakeDbException(exception);
        Assert.True(stopwatch.ElapsedMilliseconds >= waitSeconds * 1000);
        Assert.True(stopwatch.ElapsedMilliseconds <= (waitSeconds + 5) * 1000);
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithSsoTokenCachingDirectlyAsync()
    {
        Skip.When(true, ManualInteraction);

        var externalBrowserConnectionString = BaseConnectionStringWithoutAuth()
                                              + $";authenticator=externalbrowser;user={TestUser()};CLIENT_STORE_TEMPORARY_CREDENTIAL=true;poolingEnabled=false";

        using (var conn = CreateConnection(externalBrowserConnectionString))
        {
            await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.Equal(ConnectionState.Open, conn.State);
        }

        using (var conn = CreateConnection(externalBrowserConnectionString))
        {
            await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.Equal(ConnectionState.Open, conn.State);
        }
    }

    [SnowflakeFact]
    public async Task ShouldAuthenticateWithOAuthFlowAsync()
    {
        Skip.When(true, ManualOnly);

        const string Authenticator = "oauth_authorization_code"; // or "oauth_client_credentials"
        var credentialStore = new CredentialStore(initialToken: null);
        using (CredentialManagerOverride.Install(credentialStore))
        {
            using var connection = CreateConnection(ConnectionStringForOAuthFlows(Authenticator));

            await connection.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            Assert.Equal(ConnectionState.Open, connection.State);
        }
    }

    [SnowflakeFact]
    public async Task ShouldAuthenticateWithProgrammaticAccessTokenAsync()
    {
        Skip.When(true, ManualOnly);

        using var connection = CreateConnection(ConnectionStringForPat());

        await connection.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithSsoTokenCachingAsync()
    {
        Skip.When(true, ManualInteraction);

        var externalBrowserConnectionString = BaseConnectionStringWithoutAuth()
                                              + $";authenticator=externalbrowser;user={TestUser()};CLIENT_STORE_TEMPORARY_CREDENTIAL=true;poolingEnabled=false";

        using (var conn = CreateConnection(externalBrowserConnectionString))
        {
            await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.Equal(ConnectionState.Open, conn.State);
        }

        using (var conn = CreateConnection(externalBrowserConnectionString))
        {
            await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.Equal(ConnectionState.Open, conn.State);
        }
    }

    [SnowflakeFact]
    public async Task ShouldCacheMfaTokenWithPasscodeFromConnectionStringAsync()
    {
        Skip.When(true, ManualInteraction);

        using var conn = CreateConnection(
            BaseConnectionString()
            + ";authenticator=username_password_mfa;application=DuoTest;minPoolSize=0;passcode=(set proper passcode)");

        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(ConnectionState.Open, conn.State);
    }

    [SnowflakeFact]
    public async Task ShouldReauthenticateSsoWithInvalidCachedTokenAsync()
    {
        Skip.When(true, ManualInteraction);

        var credentialStore = new CredentialStore(initialToken: "wrongToken");
        using (CredentialManagerOverride.Install(credentialStore))
        {
            using var conn = CreateConnection(
                BaseConnectionStringWithoutAuth()
                + $";authenticator=externalbrowser;user={TestUser()};CLIENT_STORE_TEMPORARY_CREDENTIAL=true;");

            await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            Assert.Equal(ConnectionState.Open, conn.State);
        }

        Assert.NotEmpty(credentialStore.KeysReturningInitialToken);
        Assert.All(credentialStore.KeysReturningInitialToken, key => Assert.Contains(key, credentialStore.RemovedKeys));
    }

    [SnowflakeFact]
    public async Task ShouldFailSsoConnectionWithWrongUserAsync()
    {
        Skip.When(true, ManualInteraction);

        using var conn = CreateConnection(
            BaseConnectionStringWithoutAuth() + ";authenticator=externalbrowser;user=wrong@snowflakecomputing.com");

        var exception = await Assert.ThrowsAnyAsync<DbException>(
            () => conn.OpenAsync(TestContext.Current.CancellationToken)).ConfigureAwait(false);

        Assert.Equal(390191, AssertSnowflakeDbException(exception).ErrorCode);
    }

    [SnowflakeFact]
    public async Task ShouldFailToConnectWithExpiredOAuthTokenAsync()
    {
        Skip.When(true, NotConfiguredForCi);

        using var conn = CreateConnection(
            BaseConnectionStringWithoutAuth() + $";authenticator=oauth;token={Parameter("SNOWFLAKE_TEST_EXP_OAUTH_TOKEN")}");

        var exception = await Assert.ThrowsAnyAsync<DbException>(
            () => conn.OpenAsync(TestContext.Current.CancellationToken)).ConfigureAwait(false);

        Assert.Equal(390318, AssertSnowflakeDbException(exception).ErrorCode);
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithCorrectProxySettingFromConnectionStringAsync()
    {
        Skip.When(true, NotConfiguredForCi);

        using var conn = CreateConnection(
            BaseConnectionString()
            + $";useProxy=true;proxyHost={Parameter("PROXY_HOST")};proxyPort={Parameter("PROXY_PORT")}");

        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithCorrectProxyWithCredsSettingFromConnectionStringAsync()
    {
        Skip.When(true, NotConfiguredForCi);

        using var conn = CreateConnection(BaseConnectionString() + ";" + AuthenticatedProxy());

        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithCorrectProxySettingWithByPassListFromConnectionStringAsync()
    {
        Skip.When(true, NotConfiguredForCi);

        using var conn = CreateConnection(BaseConnectionString() + ";" + AuthenticatedProxy() + ";" + ByPassList());

        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithDifferentHttpHandlerSettingsAsync()
    {
        Skip.When(true, NotConfiguredForCi);

        var baseConnectionString = BaseConnectionString();
        var nonAuthenticatedProxy = $"useProxy=true;proxyHost={Parameter("PROXY_HOST")};proxyPort={Parameter("PROXY_PORT")}";
        string[] connectionStrings =
        [
            baseConnectionString + ";" + AuthenticatedProxy(),
            baseConnectionString,
            baseConnectionString + ";" + nonAuthenticatedProxy,
        ];
        foreach (var connectionString in connectionStrings)
        {
            using var conn = CreateConnection(connectionString);
            await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        }

        using (var conn4 = CreateConnection(
                   baseConnectionString + ";connection_timeout=20;useProxy=true;proxyHost=Invalid;proxyPort=8080;"))
        {
            var thrown = await Assert.ThrowsAnyAsync<DbException>(() => conn4.OpenAsync(TestContext.Current.CancellationToken)).ConfigureAwait(false);
            Assert.Equal(InternalErrorCode, AssertSnowflakeDbException(thrown).ErrorCode);
            Assert.Equal(ConnectionFailureSqlState, GetSqlState(thrown));
        }

        string[] laterConnectionStrings =
        [
            DisableCrlRevocationCheck(baseConnectionString + ";" + AuthenticatedProxy() + ";"),
            DisableCrlRevocationCheck(baseConnectionString),
            baseConnectionString + ";" + AuthenticatedProxy() + ";" + ByPassList(),
            baseConnectionString,
            DisableCrlRevocationCheck(baseConnectionString) + ";" + AuthenticatedProxy() + ";" + ByPassList() + ";",
            baseConnectionString + ";" + AuthenticatedProxy() + ";" + ByPassList(),
            DisableCrlRevocationCheck(baseConnectionString),
        ];
        foreach (var connectionString in laterConnectionStrings)
        {
            using var conn = CreateConnection(connectionString);
            await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        }
    }

    [SnowflakeFact]
    public async Task ShouldConnectWhenConnectionStringHasEscapedCharactersAsync()
    {
        Skip.When(true, ManualParserBreakpoint);

        await AssertBasicConnectionAsync(
            BaseConnectionString() + ";poolingEnabled=false;key1=test\'password;key2=test\"password;key3=test==password", TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldConnectWhenConnectionStringHasQuotedValuesAsync()
    {
        Skip.When(true, ManualParserBreakpoint);

        await AssertBasicConnectionAsync(
            BaseConnectionString() + ";poolingEnabled=false;key==word=value; key1=\"test;password\"; key2=\"test=password\"", TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldKeepSessionAliveWithHeartBeatAsync()
    {
        Skip.When(true, FourHours);

        using var conn = CreateConnection(BaseConnectionString() + ";poolingEnabled=false;CLIENT_SESSION_KEEP_ALIVE=true");
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        await Task.Delay(TimeSpan.FromSeconds(14430), TestContext.Current.CancellationToken).ConfigureAwait(false);
        using (var command = conn.CreateCommand())
        {
            command.CommandText = "SELECT COUNT(*) FROM DOUBLE_TABLE";
            Assert.Equal(46, await command.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
        }

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Closed, conn.State);
    }

    [SnowflakeFact]
    public async Task ShouldKeepPooledSessionAliveWithHeartBeatAsync()
    {
        Skip.When(true, FourHours);

        ClearAllPools();

        using var conn = CreateConnection(
            BaseConnectionString() + ";maxPoolSize=2;minPoolSize=0;expirationTimeout=14800;CLIENT_SESSION_KEEP_ALIVE=true");
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(1, GetCurrentPoolSize());

        using var conn1 = CreateConnection(BaseConnectionString() + ";CLIENT_SESSION_KEEP_ALIVE=true");
        await conn1.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        await Task.Delay(TimeSpan.FromSeconds(14430), TestContext.Current.CancellationToken).ConfigureAwait(false);

        using (var command = conn.CreateCommand())
        {
            command.CommandText = "SELECT COUNT(*) FROM DOUBLE_TABLE";
            Assert.Equal(46, await command.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
        }

        await conn1.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Closed, conn1.State);
        Assert.Equal(1, GetCurrentPoolSize());
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithExternalBrowserWhenUserAndPasswordEmptyAsync()
    {
        Skip.When(true, "TestConnectStringWithUserPwd, this will popup an internet browser for external login.");

        using var conn = CreateConnection(BaseConnectionStringWithoutAuth() + ";user=;password=;authenticator=externalbrowser;");
        Assert.Equal(ConnectionState.Closed, conn.State);

        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(ConnectionState.Closed, conn.State);
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithJwtUnencryptedPemFileAsync()
    {
        Skip.When(true, NotConfiguredForCi);

        await AssertJwtConnectionOpensAsync($"user={Parameter("SNOWFLAKE_TEST_JWT_USER")};private_key_file={Parameter("SNOWFLAKE_TEST_PEM_FILE")}", TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithJwtUnencryptedP8FileAsync()
    {
        Skip.When(true, NotConfiguredForCi);

        await AssertJwtConnectionOpensAsync($"user={Parameter("SNOWFLAKE_TEST_JWT_USER")};private_key_file={Parameter("SNOWFLAKE_TEST_P8_FILE")}", TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithJwtEncryptedPkFileAsync()
    {
        Skip.When(true, NotConfiguredForCi);

        await AssertJwtConnectionOpensAsync(
            $"user={Parameter("SNOWFLAKE_TEST_JWT_USER")};private_key_file={Parameter("SNOWFLAKE_TEST_PWD_PROTECTED_PK_FILE")};private_key_pwd={Parameter("SNOWFLAKE_TEST_PK_PWD")}", TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithJwtUnencryptedPkAsync()
    {
        Skip.When(true, NotConfiguredForCi);

        await AssertJwtConnectionOpensAsync($"user={Parameter("SNOWFLAKE_TEST_JWT_USER")};private_key={Parameter("SNOWFLAKE_TEST_PK_CONTENT")}", TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithJwtEncryptedPkAsync()
    {
        Skip.When(true, NotConfiguredForCi);

        await AssertJwtConnectionOpensAsync(
            $"user={Parameter("SNOWFLAKE_TEST_JWT_USER")};private_key={Parameter("SNOWFLAKE_TEST_PROTECTED_PK_CONTENT")};private_key_pwd={Parameter("SNOWFLAKE_TEST_PK_PWD")}", TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldFailJwtConnectionWithMissingConnectionSettingAsync()
    {
        Skip.When(true, NotConfiguredForCi);

        await AssertJwtConnectionFailsAsync(
            $"user={Parameter("SNOWFLAKE_TEST_JWT_USER")};private_key_pwd={Parameter("SNOWFLAKE_TEST_PK_PWD")}",
            270008, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldFailJwtConnectionWithEncryptedPkFileAndInvalidPwdAsync()
    {
        Skip.When(true, NotConfiguredForCi);

        await AssertJwtConnectionFailsAsync(
            $"user={Parameter("SNOWFLAKE_TEST_JWT_USER")};private_key_file={Parameter("SNOWFLAKE_TEST_PWD_PROTECTED_PK_FILE")};private_key_pwd=Invalid",
            270052, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldFailJwtConnectionWithEncryptedPkFileAndNoPwdAsync()
    {
        Skip.When(true, NotConfiguredForCi);

        await AssertJwtConnectionFailsAsync(
            $"user={Parameter("SNOWFLAKE_TEST_JWT_USER")};private_key_file={Parameter("SNOWFLAKE_TEST_PWD_PROTECTED_PK_FILE")}",
            270052, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldFailJwtConnectionWithWrongUserAsync()
    {
        Skip.When(true, NotConfiguredForCi);

        await AssertJwtConnectionFailsAsync($"user=WrongUser;private_key_file={Parameter("SNOWFLAKE_TEST_PEM_FILE")}", 390144, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldFailJwtEncryptedPkConnectionWithWrongUserAsync()
    {
        Skip.When(true, NotConfiguredForCi);

        await AssertJwtConnectionFailsAsync(
            $"user=WrongUser;private_key_file={Parameter("SNOWFLAKE_TEST_PWD_PROTECTED_PK_FILE")};private_key_pwd={Parameter("SNOWFLAKE_TEST_PK_PWD")}",
            390144, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldConnectWithValidOAuthAsync()
    {
        Skip.When(true, NotConfiguredForCi);

        using var conn = CreateConnection(
            BaseConnectionStringWithoutAuth() + $";authenticator=oauth;token={Parameter("SNOWFLAKE_TEST_OAUTH_TOKEN")}");

        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(ConnectionState.Open, conn.State);
    }

    [SnowflakeFact]
    public async Task ShouldOpenMultipleConnectionsInParallelAsync()
    {
        Skip.When(true, NotConfiguredForCi);

        var baseConnectionString = BaseConnectionString() + ";CONNECTION_TIMEOUT=30;";
        var authenticatedProxy =
            $"useProxy =true;proxyHost={Parameter("AUTH_PROXY_HOST")};proxyPort={Parameter("AUTH_PROXY_PORT")};proxyUser={Parameter("AUTH_PROXY_USER")};proxyPassword={Parameter("AUTH_PROXY_PWD")};";
        var byPassList = ByPassList() + ";";
        string[] connectionStrings =
        [
            baseConnectionString,
            DisableCrlRevocationCheck(baseConnectionString),
            baseConnectionString + authenticatedProxy,
            DisableCrlRevocationCheck(baseConnectionString + authenticatedProxy),
            baseConnectionString + authenticatedProxy + byPassList,
            DisableCrlRevocationCheck(baseConnectionString + authenticatedProxy + byPassList),
        ];

        var tasks = Enumerable.Range(0, 450).Select(i => Task.Run(async () =>
        {
            using var conn = CreateConnection(connectionStrings[i % connectionStrings.Length]);
            await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            using var command = conn.CreateCommand();
            command.CommandText = "SELECT 1";
            await command.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        }));

        await Task.WhenAll(tasks).ConfigureAwait(false);
    }

    private async Task AssertBasicConnectionAsync(string connectionString, CancellationToken cancellationToken)
    {
        using var conn = CreateDbConnection(connectionString);
        await conn.OpenAsync(cancellationToken).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, conn.State);

        Assert.Equal(OldDriverDefaultRetryTimeoutSeconds, conn.ConnectionTimeout);
        Assert.Equal("", conn.DataSource);

        var serverVersion = conn.ServerVersion;
        if (!string.IsNullOrEmpty(serverVersion))
        {
            var versionElements = serverVersion.Split('.');
            Assert.Equal(3, versionElements.Length);
        }

        await CloseDbConnectionAsync(conn, cancellationToken).ConfigureAwait(false);
        Assert.Equal(ConnectionState.Closed, conn.State);
    }

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

    private async Task AssertJwtConnectionOpensAsync(string jwtSettings, CancellationToken cancellationToken)
    {
        using var conn = CreateConnection(BaseConnectionStringWithoutAuth() + ";authenticator=snowflake_jwt;" + jwtSettings);

        await conn.OpenAsync(cancellationToken).ConfigureAwait(false);

        Assert.Equal(ConnectionState.Open, conn.State);
    }

    private async Task AssertJwtConnectionFailsAsync(string jwtSettings, int expectedErrorCode, CancellationToken cancellationToken)
    {
        using var conn = CreateConnection(BaseConnectionStringWithoutAuth() + ";authenticator=snowflake_jwt;" + jwtSettings);

        var exception = await Assert.ThrowsAnyAsync<DbException>(
            () => conn.OpenAsync(cancellationToken)).ConfigureAwait(false);

        Assert.Equal(expectedErrorCode, AssertSnowflakeDbException(exception).ErrorCode);
    }

    private string OktaConnectionString() =>
        BaseConnectionStringWithoutAuth()
        + $";authenticator={Parameter("SNOWFLAKE_TEST_OKTA_URL")};user={Parameter("SNOWFLAKE_TEST_OKTA_USER")};password={Parameter("SNOWFLAKE_TEST_OKTA_PASSWORD")};";

    private string ConnectionStringForPat()
    {
        var builder = new DbConnectionStringBuilder { ConnectionString = BaseConnectionStringWithoutAuth() };
        builder["role"] = "ANALYST";
        return builder.ConnectionString
               + $";authenticator=programmatic_access_token;user={TestUser()};token={Parameter("SNOWFLAKE_TEST_PROGRAMMATIC_ACCESS_TOKEN")};poolingEnabled=false;";
    }

    private string ConnectionStringForOAuthFlows(string authenticator)
    {
        var builder = new DbConnectionStringBuilder { ConnectionString = BaseConnectionStringWithoutAuth() };
        builder["authenticator"] = authenticator;
        builder["user"] = TestUser();
        builder["oauthClientId"] = Parameter("SNOWFLAKE_TEST_OAUTH_CLIENT_ID");
        builder["oauthClientSecret"] = Parameter("SNOWFLAKE_TEST_OAUTH_CLIENT_SECRET");
        builder["oauthScope"] = Parameter("SNOWFLAKE_TEST_OAUTH_SCOPE");
        builder["oauthTokenRequestUrl"] = Parameter("SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL");
        builder["poolingEnabled"] = "false";
        switch (authenticator)
        {
            case "oauth_authorization_code":
                builder["oauthRedirectUri"] = Parameter("SNOWFLAKE_TEST_OAUTH_REDIRECT_URI");
                builder["oauthAuthorizationUrl"] = Parameter("SNOWFLAKE_TEST_OAUTH_AUTHORIZATION_URL");
                break;
            case "oauth_client_credentials":
                break;
            default:
                throw new ArgumentException($"Unknown authenticator {authenticator}", nameof(authenticator));
        }

        return builder.ConnectionString;
    }

    private string AuthenticatedProxy() =>
        $"useProxy=true;proxyHost={Parameter("AUTH_PROXY_HOST")};proxyPort={Parameter("AUTH_PROXY_PORT")};proxyUser={Parameter("AUTH_PROXY_USER")};proxyPassword={Parameter("AUTH_PROXY_PWD")}";

    private string ByPassList() => $"nonProxyHosts=*.foo.com %7C{ResolveHost()}|localhost";

    private static string DisableCrlRevocationCheck(string connectionString)
    {
        var modified = Regex.Replace(
            connectionString,
            "certRevocationCheckMode=enabled",
            "certRevocationCheckMode=disabled",
            RegexOptions.IgnoreCase);
        return connectionString != modified ? modified : connectionString + ";certRevocationCheckMode=disabled;";
    }

    private static string Parameter(string key) => ParametersReader.Get(key) ?? string.Empty;

    private string TestUser() => Setting(BaseConnectionString(), "user");

    private string ResolveHost()
    {
        var builder = new DbConnectionStringBuilder { ConnectionString = BaseConnectionString() };
        return builder.TryGetValue("host", out var host) && host is string text
            ? text
            : $"{Setting(BaseConnectionString(), "account")}.snowflakecomputing.com";
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

    private ISnowflakeDbConnection CreateConnection(string connectionString) =>
        Fixture.Factory.Create(Output, connectionString);

    private static string Setting(string connectionString, string key)
    {
        var builder = new DbConnectionStringBuilder { ConnectionString = connectionString };
        Assert.True(builder.TryGetValue(key, out var value), $"Connection string has no '{key}'");
        return Assert.IsType<string>(value);
    }

    private static bool GetPooling(string connectionString) =>
        TestDriver.Current == TestDriver.Kind.Old
            ? OldSnowflakeDbConnectionPool.GetPool(connectionString).GetPooling()
            : NewSnowflakeDbConnectionPool.GetPooling(connectionString);

    private static void ClearAllPools()
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
            OldSnowflakeDbConnectionPool.ClearAllPools();
        else
            NewSnowflakeDbConnectionPool.ClearAllPools();
    }

    private static int GetCurrentPoolSize() =>
        TestDriver.Current == TestDriver.Kind.Old
            ? OldSnowflakeDbConnectionPool.GetCurrentPoolSize()
            : NewSnowflakeDbConnectionPool.GetCurrentPoolSize();

    private static DbException AssertSnowflakeDbException(Exception exception)
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
            return Assert.IsType<OldSnowflakeDbException>(exception);
        return Assert.IsType<NewSnowflakeDbException>(exception);
    }

    private static string? GetSqlState(DbException exception) =>
        exception switch
        {
            OldSnowflakeDbException old => old.SqlState,
            NewSnowflakeDbException current => current.SqlState,
            _ => null,
        };

    [CollectionDefinition(nameof(SFConnectionManualTest), DisableParallelization = true)]
    public sealed class SerialCollection
    {
    }
}
