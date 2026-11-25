using Snowflake.Data.Client;
using Snowflake.Data.Core.Session;

namespace Snowflake.Data.Tests.Session;

[Collection(SnowflakeDbConnectionPoolStaticsCollection.Name)]
[Trait("Category", "Unit")]
public sealed class SnowflakeDbConnectionPoolStaticsTest : IDisposable
{
    public SnowflakeDbConnectionPoolStaticsTest() => SnowflakeDbConnectionPool.ClearAllPools();

    public void Dispose() => SnowflakeDbConnectionPool.ClearAllPools();

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void GetPoolingStaysTrueWhenAPartitionDisablesPooling()
    {
        SnowflakeDbConnectionPool.Default.GetOrCreate(
            PoolKeyFactory.From("account=pooling-off"),
            ConnectionPoolConfig.Default with { PoolingEnabled = false });

        Assert.False(SnowflakeDbConnectionPool.GetPooling("account=pooling-off"));
        Assert.True(SnowflakeDbConnectionPool.GetPooling());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void EmptyRegistryReportsTheDefaultMaxPoolSizeTimeoutAndZeroCurrentSize()
    {
        Assert.Equal(ConnectionPoolConfig.Default.MaxPoolSize, SnowflakeDbConnectionPool.GetMaxPoolSize());
        Assert.Equal(
            PoolTimeout.ToWholeSeconds(ConnectionPoolConfig.Default.ExpirationTimeout, mapInfiniteToMinusOne: true),
            SnowflakeDbConnectionPool.GetTimeout());
        Assert.Equal(0, SnowflakeDbConnectionPool.GetCurrentPoolSize());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void PerKeyStaticsForwardToThePartitionOnTheDefaultRegistry()
    {
        const string connectionString = "account=forward";
        using var password = TestSecureString.From("alpha");
        var config = new ConnectionPoolConfig
        {
            MaxPoolSize = 7,
            MinPoolSize = 3,
            ChangedSession = ChangedSessionBehavior.OriginalPool,
            WaitingForIdleSessionTimeout = TimeSpan.FromSeconds(9),
            ExpirationTimeout = TimeSpan.FromMinutes(2),
            PoolingEnabled = false,
            ConnectionTimeout = TimeSpan.FromSeconds(12),
        };
        var partition = SnowflakeDbConnectionPool.Default.GetOrCreate(
            PoolKeyFactory.From(connectionString, password),
            config);

        Assert.Same(partition, SnowflakeDbConnectionPool.GetPoolPartition(connectionString, password));
        Assert.Equal(7, SnowflakeDbConnectionPool.GetMaxPoolSize(connectionString, password));
        Assert.Equal(3, SnowflakeDbConnectionPool.GetMinPoolSize(connectionString, password));
        Assert.Equal(0, SnowflakeDbConnectionPool.GetCurrentPoolSize(connectionString, password));
        Assert.False(SnowflakeDbConnectionPool.GetPooling(connectionString, password));
        Assert.Equal(120, SnowflakeDbConnectionPool.GetExpirationTimeout(connectionString, password));
        Assert.Equal(12, SnowflakeDbConnectionPool.GetConnectionTimeout(connectionString, password));
        Assert.Equal(9, SnowflakeDbConnectionPool.GetWaitForIdleSessionTimeout(connectionString, password));
        Assert.Equal(ChangedSessionBehavior.OriginalPool, SnowflakeDbConnectionPool.GetChangedSession(connectionString, password));

        SnowflakeDbConnectionPool.ClearPool(connectionString, password);

        Assert.Same(partition, SnowflakeDbConnectionPool.GetPoolPartition(connectionString, password));
#pragma warning disable CS0618
        Assert.Same(partition, SnowflakeDbConnectionPool.GetPool(connectionString, password).Partition);
#pragma warning restore CS0618
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void OauthClientSecretAndTokenSelectDifferentPartitions()
    {
        const string connectionString = "account=oauth-or-token";
        using var alpha = TestSecureString.From("alpha");
        using var alphaAgain = TestSecureString.From("alpha");
        using var beta = TestSecureString.From("beta");

        var absent = SnowflakeDbConnectionPool.GetPoolPartition(connectionString);
        var oauth = SnowflakeDbConnectionPool.GetPoolPartition(
            connectionString, password: null, oauthClientSecret: alpha);
        var oauthAgain = SnowflakeDbConnectionPool.GetPoolPartition(
            connectionString, password: null, oauthClientSecret: alphaAgain);
        var token = SnowflakeDbConnectionPool.GetPoolPartition(
            connectionString, password: null, token: alpha);
        var otherOauth = SnowflakeDbConnectionPool.GetPoolPartition(
            connectionString, password: null, oauthClientSecret: beta);

        Assert.Same(oauth, oauthAgain);
        Assert.NotSame(absent, oauth);
        Assert.NotSame(oauth, token);
        Assert.NotSame(oauth, otherOauth);
#pragma warning disable CS0618
        Assert.Same(oauth, SnowflakeDbConnectionPool.GetPool(connectionString, null, alpha).Partition);
        Assert.Same(token, SnowflakeDbConnectionPool.GetPool(connectionString, null, null, alpha).Partition);
#pragma warning restore CS0618
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void NullConnectionStringThrowsArgumentNullException()
    {
        AssertNullConnectionString(() => SnowflakeDbConnectionPool.GetPoolPartition(null!));
        AssertNullConnectionString(() => SnowflakeDbConnectionPool.GetPoolPartition(null!, null, null, null));
        AssertNullConnectionString(() => SnowflakeDbConnectionPool.GetPooling(null!));
        AssertNullConnectionString(() => SnowflakeDbConnectionPool.GetMinPoolSize(null!));
        AssertNullConnectionString(() => SnowflakeDbConnectionPool.GetMaxPoolSize(null!));
        AssertNullConnectionString(() => SnowflakeDbConnectionPool.GetCurrentPoolSize(null!));
        AssertNullConnectionString(() => SnowflakeDbConnectionPool.GetExpirationTimeout(null!));
        AssertNullConnectionString(() => SnowflakeDbConnectionPool.GetConnectionTimeout(null!));
        AssertNullConnectionString(() => SnowflakeDbConnectionPool.GetWaitForIdleSessionTimeout(null!));
        AssertNullConnectionString(() => SnowflakeDbConnectionPool.ClearPool(null!));
        AssertNullConnectionString(() => SnowflakeDbConnectionPool.GetChangedSession(null!));
#pragma warning disable CS0618
        AssertNullConnectionString(() => SnowflakeDbConnectionPool.GetPool(null!));
        AssertNullConnectionString(() => SnowflakeDbConnectionPool.GetPool(null!, null));
#pragma warning restore CS0618
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ClearAllPoolsDetachesPartitionsFromTheDefaultRegistry()
    {
        const string connectionString = "account=detach";
        var first = SnowflakeDbConnectionPool.GetPoolPartition(connectionString);

        SnowflakeDbConnectionPool.ClearAllPools();

        Assert.NotSame(first, SnowflakeDbConnectionPool.GetPoolPartition(connectionString));
        Assert.Equal(0, SnowflakeDbConnectionPool.GetCurrentPoolSize());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DifferingPartitionsThrowFromThePublicAggregate()
    {
        // ToPublicException builds a SnowflakeDbException, and that constructor throws NotImplementedException.
        SnowflakeDbConnectionPool.Default.GetOrCreate(
            PoolKeyFactory.From("account=max-a"),
            ConnectionPoolConfig.Default with { MaxPoolSize = 3 });
        SnowflakeDbConnectionPool.Default.GetOrCreate(
            PoolKeyFactory.From("account=max-b"),
            ConnectionPoolConfig.Default with { MaxPoolSize = 4 });

        var exception = Assert.Throws<NotImplementedException>(() => SnowflakeDbConnectionPool.GetMaxPoolSize());

        Assert.Equal("TODO next milestone", exception.Message);
    }

    private static void AssertNullConnectionString(Action call)
    {
        var exception = Assert.Throws<ArgumentNullException>(call);
        Assert.Equal("connectionString", exception.ParamName);
    }
}
