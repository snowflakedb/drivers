using Snowflake.Data.Core;
using Snowflake.Data.Core.Errors;
using Snowflake.Data.Core.Session;

namespace Snowflake.Data.Tests.Session;

[Trait("Category", "Unit")]
public sealed class ConnectionPoolConfigParserTest
{
    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void AbsentKeysUseTheDefaults()
    {
        var config = ConnectionPoolConfigParser.Parse("account=acct");

        Assert.Equal(ConnectionPoolConfig.Default, config);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void EmptyValuesUseTheDefaults()
    {
        var config = ConnectionPoolConfigParser.Parse("maxPoolSize=\"\";minPoolSize=\"\";poolingEnabled=\"\"");

        Assert.Equal(ConnectionPoolConfig.Default, config);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void KeysAndEnumValuesAreCaseInsensitive()
    {
        var config = ConnectionPoolConfigParser.Parse(
            "maxpoolsize=4;MINPOOLSIZE=0;changedSession=originalpool;poolingenabled=FALSE;expirationTimeout=2m;waitingForIdleSessionTimeout=1500ms;connection_timeout=12s");

        Assert.Equal(4, config.MaxPoolSize);
        Assert.Equal(0, config.MinPoolSize);
        Assert.Equal(ChangedSessionBehavior.OriginalPool, config.ChangedSession);
        Assert.False(config.PoolingEnabled);
        Assert.Equal(TimeSpan.FromMinutes(2), config.ExpirationTimeout);
        Assert.Equal(TimeSpan.FromMilliseconds(1500), config.WaitingForIdleSessionTimeout);
        Assert.Equal(TimeSpan.FromSeconds(12), config.ConnectionTimeout);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData("Destroy", ChangedSessionBehavior.Destroy)]
    [InlineData("1", ChangedSessionBehavior.Destroy)]
    [InlineData("originalpool", ChangedSessionBehavior.OriginalPool)]
    [InlineData("0", ChangedSessionBehavior.OriginalPool)]
    public void ChangedSessionAcceptsNamesAndLegacyNumbers(string raw, ChangedSessionBehavior expected)
    {
        var config = ConnectionPoolConfigParser.Parse($"changedSession={raw}");

        Assert.Equal(expected, config.ChangedSession);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData("15", 15)]
    [InlineData("15s", 15)]
    [InlineData("15S", 15)]
    [InlineData("2m", 120)]
    [InlineData("500ms", 0.5)]
    public void TimeoutsAcceptLegacyUnits(string raw, double seconds)
    {
        var config = ConnectionPoolConfigParser.Parse($"expirationTimeout={raw}");

        Assert.Equal(TimeSpan.FromSeconds(seconds), config.ExpirationTimeout);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void NegativeExpirationAndConnectionTimeoutNeverElapse()
    {
        var config = ConnectionPoolConfigParser.Parse("expirationTimeout=-1;connection_timeout=-5s");

        Assert.Equal(Timeout.InfiniteTimeSpan, config.ExpirationTimeout);
        Assert.Equal(Timeout.InfiniteTimeSpan, config.ConnectionTimeout);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ZeroConnectionTimeoutNeverElapses()
    {
        var config = ConnectionPoolConfigParser.Parse("connection_timeout=0");

        Assert.Equal(Timeout.InfiniteTimeSpan, config.ConnectionTimeout);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ZeroIdleWaitIsKept()
    {
        var config = ConnectionPoolConfigParser.Parse("waitingForIdleSessionTimeout=0");

        Assert.Equal(TimeSpan.Zero, config.WaitingForIdleSessionTimeout);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData("maxPoolSize=0", "Invalid value of parameter MAXPOOLSIZE.")]
    [InlineData("maxPoolSize=-1", "Invalid value of parameter MAXPOOLSIZE.")]
    [InlineData("maxPoolSize=abc", "Invalid value of parameter MAXPOOLSIZE.")]
    [InlineData("maxPoolSize=2147483648", "Invalid value of parameter MAXPOOLSIZE.")]
    [InlineData("minPoolSize=-1", "Invalid value of parameter MINPOOLSIZE.")]
    [InlineData("changedSession=recycle", "Invalid value of parameter CHANGEDSESSION.")]
    [InlineData("changedSession=2", "Invalid value of parameter CHANGEDSESSION.")]
    [InlineData("poolingEnabled=yes", "Invalid value of parameter POOLINGENABLED.")]
    [InlineData("expirationTimeout=1h", "Invalid value of parameter EXPIRATIONTIMEOUT.")]
    [InlineData("expirationTimeout=9999999999", "Invalid value of parameter EXPIRATIONTIMEOUT.")]
    [InlineData("waitingForIdleSessionTimeout=-1", "WAITINGFORIDLESESSIONTIMEOUT cannot be infinite.")]
    [InlineData("connection_timeout=1h", "Invalid value of parameter CONNECTION_TIMEOUT.")]
    [InlineData("retry_timeout=1h", "Invalid value of parameter RETRY_TIMEOUT.")]
    public void InvalidValuesNameTheKey(string connectionString, string message)
    {
        var exception = Assert.Throws<SnowflakeDriverException>(() => ConnectionPoolConfigParser.Parse(connectionString));

        Assert.Equal(SFError.INVALID_CONNECTION_STRING, exception.Error);
        Assert.Equal(message, Assert.Single(exception.Args));
        Assert.DoesNotContain(connectionString, exception.ToString(), StringComparison.Ordinal);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void MinPoolSizeAboveMaxPoolSizeNamesBothKeys()
    {
        var exception = Assert.Throws<SnowflakeDriverException>(() =>
            ConnectionPoolConfigParser.Parse("minPoolSize=5;maxPoolSize=4"));

        Assert.Equal(SFError.INVALID_CONNECTION_STRING, exception.Error);
        Assert.Equal("MinPoolSize cannot be greater than MaxPoolSize.", Assert.Single(exception.Args));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void MinPoolSizeEqualToMaxPoolSizeIsAccepted()
    {
        var config = ConnectionPoolConfigParser.Parse("minPoolSize=4;maxPoolSize=4");

        Assert.Equal(4, config.MinPoolSize);
        Assert.Equal(4, config.MaxPoolSize);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData("authenticator=externalbrowser")]
    [InlineData("authenticator=EXTERNALBROWSER")]
    [InlineData("authenticator=oauth_authorization_code")]
    [InlineData("authenticator=snowflake_jwt;private_key_file=/tmp/key.p8")]
    public void RestrictedAuthenticatorsDisablePoolingWhenTheKeyIsAbsent(string connectionString)
    {
        var config = ConnectionPoolConfigParser.Parse(connectionString);

        Assert.False(config.PoolingEnabled);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ExplicitPoolingStaysEnabledForARestrictedAuthenticator()
    {
        var config = ConnectionPoolConfigParser.Parse("authenticator=externalbrowser;poolingEnabled=true");

        Assert.True(config.PoolingEnabled);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void KeyPairWithAPasswordKeepsThePoolingDefault()
    {
        var config = ConnectionPoolConfigParser.Parse(
            "authenticator=snowflake_jwt;private_key_file=/tmp/key.p8;private_key_pwd=secret");

        Assert.True(config.PoolingEnabled);
        Assert.DoesNotContain("secret", config.ToString(), StringComparison.Ordinal);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void OAuthAuthorizationCodeWithPoolingAndNoUserFails()
    {
        var exception = Assert.Throws<SnowflakeDriverException>(() =>
            ConnectionPoolConfigParser.Parse("authenticator=oauth_authorization_code;poolingEnabled=true"));

        Assert.Equal(SFError.INVALID_CONNECTION_STRING, exception.Error);
        Assert.Equal(
            "You cannot enable pooling for oauth authorization code authentication without specifying a user in the connection string.",
            Assert.Single(exception.Args));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void OAuthAuthorizationCodeWithPoolingDisabledAndNoUserSucceeds()
    {
        var config = ConnectionPoolConfigParser.Parse("authenticator=oauth_authorization_code;poolingEnabled=false");

        Assert.False(config.PoolingEnabled);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void OAuthWithoutAUserFailsBeforeAPoolSizeError()
    {
        var exception = Assert.Throws<SnowflakeDriverException>(() =>
            ConnectionPoolConfigParser.Parse(
                "authenticator=oauth_authorization_code;poolingEnabled=true;minPoolSize=5;maxPoolSize=4"));

        Assert.Equal(
            "You cannot enable pooling for oauth authorization code authentication without specifying a user in the connection string.",
            Assert.Single(exception.Args));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData("connection_timeout=600", 300)]
    [InlineData("retry_timeout=100", 300)]
    [InlineData("connection_timeout=600;retry_timeout=100", 300)]
    [InlineData("connection_timeout=600;retry_timeout=0", 600)]
    [InlineData("connection_timeout=100", 100)]
    public void ConnectionTimeoutIsCappedByTheRetryTimeout(string connectionString, int seconds)
    {
        var config = ConnectionPoolConfigParser.Parse(connectionString);

        Assert.Equal(TimeSpan.FromSeconds(seconds), config.ConnectionTimeout);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void EmptyKeyPairPasswordRestrictsPooling()
    {
        var config = ConnectionPoolConfigParser.Parse(
            "authenticator=snowflake_jwt;private_key_file=/tmp/key.p8;private_key_pwd=\"\"");

        Assert.False(config.PoolingEnabled);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ExplicitFalseKeepsPoolingDisabledForARestrictedAuthenticator()
    {
        var config = ConnectionPoolConfigParser.Parse("authenticator=externalbrowser;poolingEnabled=false");

        Assert.False(config.PoolingEnabled);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void OAuthAuthorizationCodeWithPoolingAndAUserSucceeds()
    {
        var config = ConnectionPoolConfigParser.Parse(
            "authenticator=oauth_authorization_code;poolingEnabled=true;user=alice");

        Assert.True(config.PoolingEnabled);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void AConnectionStringThatCannotBeParsedNamesNoSecret()
    {
        const string connectionString = "password=\"hidden";

        var exception = Assert.Throws<SnowflakeDriverException>(() => ConnectionPoolConfigParser.Parse(connectionString));

        Assert.Equal(SFError.INVALID_CONNECTION_STRING, exception.Error);
        Assert.IsType<ArgumentException>(exception.InnerException);
        Assert.DoesNotContain("hidden", exception.ToString(), StringComparison.Ordinal);
        Assert.DoesNotContain(connectionString, Assert.Single(exception.Args).ToString(), StringComparison.Ordinal);
    }
}
