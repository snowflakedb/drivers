using Snowflake.Data.Client;
using Snowflake.Data.Core.Session;

namespace Snowflake.Data.Tests.Session;

[Trait("Category", "Unit")]
public sealed class SnowflakeDbConnectionPoolPartitionTest
{
    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DefaultConfigIsReflectedByThePartition()
    {
        var key = PoolKeyFactory.From("account=defaults");
        var partition = new SnowflakeDbConnectionPool(key, ConnectionPoolConfig.Default);

        Assert.Equal(key, partition.Key);
        Assert.Equal(10, partition.MaxPoolSize);
        Assert.Equal(2, partition.MinPoolSize);
        Assert.Equal(0, partition.CurrentPoolSize);
        Assert.True(partition.PoolingEnabled);
        Assert.Equal(TimeSpan.FromHours(1), partition.ExpirationTimeout);
        Assert.Equal(TimeSpan.FromSeconds(300), partition.ConnectionTimeout);
        Assert.Equal(TimeSpan.FromSeconds(30), partition.WaitingForIdleSessionTimeout);
        Assert.Equal(ChangedSessionBehavior.Destroy, partition.ChangedSession);
        Assert.Equal(ConnectionPoolConfig.Default, new ConnectionPoolConfig());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CustomConfigIsReflectedAndClearLeavesTheSizeAtZero()
    {
        var key = PoolKeyFactory.From("account=custom");
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
        var partition = new SnowflakeDbConnectionPool(key, config);

        Assert.Equal(key, partition.Key);
        Assert.Equal(7, partition.MaxPoolSize);
        Assert.Equal(3, partition.MinPoolSize);
        Assert.Equal(0, partition.CurrentPoolSize);
        Assert.False(partition.PoolingEnabled);
        Assert.Equal(TimeSpan.FromMinutes(2), partition.ExpirationTimeout);
        Assert.Equal(TimeSpan.FromSeconds(12), partition.ConnectionTimeout);
        Assert.Equal(TimeSpan.FromSeconds(9), partition.WaitingForIdleSessionTimeout);
        Assert.Equal(ChangedSessionBehavior.OriginalPool, partition.ChangedSession);

        partition.Clear();

        Assert.Equal(0, partition.CurrentPoolSize);
    }
}
