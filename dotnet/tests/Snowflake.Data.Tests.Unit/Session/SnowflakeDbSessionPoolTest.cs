using Snowflake.Data.Client;
using Snowflake.Data.Core.Session;

#pragma warning disable CS0618

namespace Snowflake.Data.Tests.Session;

[Collection(SnowflakeDbConnectionPoolStaticsCollection.Name)]
[Trait("Category", "Unit")]
public sealed class SnowflakeDbSessionPoolTest : IDisposable
{
    public SnowflakeDbSessionPoolTest() => SnowflakeDbConnectionPool.ClearAllPools();

    public void Dispose() => SnowflakeDbConnectionPool.ClearAllPools();

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ForwardsEveryMemberToThePartition()
    {
        var partition = new FakePartition
        {
            PoolingEnabled = false,
            MinPoolSize = 4,
            MaxPoolSize = 11,
            CurrentPoolSize = 6,
            ExpirationTimeout = TimeSpan.FromSeconds(15),
            ConnectionTimeout = TimeSpan.FromSeconds(8),
            WaitingForIdleSessionTimeout = TimeSpan.FromSeconds(3),
            ChangedSession = ChangedSessionBehavior.OriginalPool,
        };
        var pool = new SnowflakeDbSessionPool(partition);

        Assert.False(pool.GetPooling());
        Assert.Equal(4, pool.GetMinPoolSize());
        Assert.Equal(11, pool.GetMaxPoolSize());
        Assert.Equal(6, pool.GetCurrentPoolSize());
        Assert.Equal(15, pool.GetExpirationTimeout());
        Assert.Equal(8, pool.GetConnectionTimeout());
        Assert.Equal(3, pool.GetWaitForIdleSessionTimeout());
        Assert.Equal(ChangedSessionBehavior.OriginalPool, pool.GetChangedSession());
        Assert.Same(partition, pool.Partition);

        pool.ClearPool();

        Assert.Equal(1, partition.ClearCount);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TimeoutsConvertToWholeSeconds()
    {
        var partition = new FakePartition
        {
            ExpirationTimeout = TimeSpan.FromSeconds(10.9),
            ConnectionTimeout = TimeSpan.FromSeconds(-2.2),
            WaitingForIdleSessionTimeout = TimeSpan.FromSeconds(-2.2),
        };
        var pool = new SnowflakeDbSessionPool(partition);

        Assert.Equal(10, pool.GetExpirationTimeout());
        Assert.Equal(-2, pool.GetConnectionTimeout());
        Assert.Equal(-2, pool.GetWaitForIdleSessionTimeout());

        partition.ExpirationTimeout = Timeout.InfiniteTimeSpan;
        partition.ConnectionTimeout = Timeout.InfiniteTimeSpan;
        partition.WaitingForIdleSessionTimeout = Timeout.InfiniteTimeSpan;

        Assert.Equal(-1, pool.GetExpirationTimeout());
        Assert.Equal(-1, pool.GetConnectionTimeout());
        Assert.Equal(0, pool.GetWaitForIdleSessionTimeout());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void GetPoolWrapsTheSamePartitionAsGetPoolPartition()
    {
        const string connectionString = "account=wrap";
        var partition = SnowflakeDbConnectionPool.GetPoolPartition(connectionString);

        var sessionPool = SnowflakeDbConnectionPool.GetPool(connectionString);
        var withNullPassword = SnowflakeDbConnectionPool.GetPool(connectionString, null);

        Assert.Same(partition, sessionPool.Partition);
        Assert.Same(partition, withNullPassword.Partition);
    }

    private sealed class FakePartition : ISnowflakeDbConnectionPoolPartition
    {
        public int MaxPoolSize { get; set; }

        public int MinPoolSize { get; set; }

        public int CurrentPoolSize { get; set; }

        public bool PoolingEnabled { get; set; }

        public TimeSpan ExpirationTimeout { get; set; }

        public TimeSpan ConnectionTimeout { get; set; }

        public TimeSpan WaitingForIdleSessionTimeout { get; set; }

        public ChangedSessionBehavior ChangedSession { get; set; }

        public int ClearCount { get; private set; }

        public void Clear() => ClearCount++;
    }
}

#pragma warning restore CS0618
