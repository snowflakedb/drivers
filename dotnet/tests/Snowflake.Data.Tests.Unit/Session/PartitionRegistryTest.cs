using System.Security;
using Snowflake.Data.Client;
using Snowflake.Data.Core;
using Snowflake.Data.Core.Errors;
using Snowflake.Data.Core.Session;

namespace Snowflake.Data.Tests.Session;

[Trait("Category", "Unit")]
public sealed class PartitionRegistryTest
{
    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void SameKeyReturnsTheSamePartition()
    {
        var registry = new SnowflakeDbConnectionPool.PartitionRegistry();
        var first = registry.GetOrCreate(PoolKeyFactory.From("account=same"));
        var second = registry.GetOrCreate(PoolKeyFactory.From("account=same"));
        var replaced = registry.GetOrCreate(
            PoolKeyFactory.From("account=same"),
            ConnectionPoolConfig.Default with { MaxPoolSize = 9 });

        Assert.Same(first, second);
        Assert.Same(first, replaced);
        Assert.Equal(ConnectionPoolConfig.Default.MaxPoolSize, replaced.MaxPoolSize);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DifferentConnectionStringsReturnDifferentPartitions()
    {
        var registry = new SnowflakeDbConnectionPool.PartitionRegistry();

        var lower = registry.GetOrCreate(PoolKeyFactory.From("account=one"));
        var other = registry.GetOrCreate(PoolKeyFactory.From("account=two"));
        var upper = registry.GetOrCreate(PoolKeyFactory.From("Account=one"));

        Assert.NotSame(lower, other);
        Assert.NotSame(lower, upper);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void SecretsSelectThePartition()
    {
        var registry = new SnowflakeDbConnectionPool.PartitionRegistry();
        const string connectionString = "account=secrets";
        using var alpha = TestSecureString.From("alpha");
        using var alphaAgain = TestSecureString.From("alpha");
        using var beta = TestSecureString.From("beta");
        using var empty = new SecureString();

        var absent = registry.GetOrCreate(PoolKeyFactory.From(connectionString));
        var nullSecrets = registry.GetOrCreate(PoolKeyFactory.From(connectionString, null, null, null));
        var emptySecrets = registry.GetOrCreate(PoolKeyFactory.From(connectionString, empty, empty, empty));
        var withAlpha = registry.GetOrCreate(PoolKeyFactory.From(connectionString, alpha));
        var withAlphaAgain = registry.GetOrCreate(PoolKeyFactory.From(connectionString, alphaAgain));
        var withBeta = registry.GetOrCreate(PoolKeyFactory.From(connectionString, beta));
        var alphaAsToken = registry.GetOrCreate(PoolKeyFactory.From(connectionString, token: alpha));

        Assert.Same(absent, nullSecrets);
        Assert.Same(absent, emptySecrets);
        Assert.Same(withAlpha, withAlphaAgain);
        Assert.NotSame(absent, withAlpha);
        Assert.NotSame(withAlpha, withBeta);
        Assert.NotSame(withAlpha, alphaAsToken);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void EmptyRegistrySumsToZeroAndAggregateReturnsTheDefault()
    {
        var registry = new SnowflakeDbConnectionPool.PartitionRegistry();

        Assert.Equal(0, registry.CurrentPoolSize());
        Assert.Equal(ConnectionPoolConfig.Default.MaxPoolSize, registry.UniformMaxPoolSize());
        Assert.Equal(
            PoolTimeout.ToWholeSeconds(ConnectionPoolConfig.Default.ExpirationTimeout, mapInfiniteToMinusOne: true),
            registry.UniformTimeoutSeconds());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void UniformReturnsTheSharedValue()
    {
        var registry = new SnowflakeDbConnectionPool.PartitionRegistry();
        registry.GetOrCreate(PoolKeyFactory.From("account=a"), ConnectionPoolConfig.Default with { MaxPoolSize = 4 });
        registry.GetOrCreate(PoolKeyFactory.From("account=b"), ConnectionPoolConfig.Default with { MaxPoolSize = 4 });

        Assert.Equal(4, registry.UniformMaxPoolSize());

        var timeouts = new SnowflakeDbConnectionPool.PartitionRegistry();
        timeouts.GetOrCreate(
            PoolKeyFactory.From("account=t1"),
            ConnectionPoolConfig.Default with { ExpirationTimeout = TimeSpan.FromSeconds(60) });
        timeouts.GetOrCreate(
            PoolKeyFactory.From("account=t2"),
            ConnectionPoolConfig.Default with { ExpirationTimeout = TimeSpan.FromSeconds(60.9) });

        Assert.Equal(60, timeouts.UniformTimeoutSeconds());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DifferingValuesThrowSnowflakeDriverException()
    {
        var sizes = new SnowflakeDbConnectionPool.PartitionRegistry();
        sizes.GetOrCreate(PoolKeyFactory.From("account=s1"), ConnectionPoolConfig.Default with { MaxPoolSize = 3 });
        sizes.GetOrCreate(PoolKeyFactory.From("account=s2"), ConnectionPoolConfig.Default with { MaxPoolSize = 8 });

        var sizeException = Assert.Throws<SnowflakeDriverException>(() => sizes.UniformMaxPoolSize());
        Assert.Equal(SFError.INCONSISTENT_RESULT_ERROR, sizeException.Error);
        Assert.Equal("Multiple pools have different Max Pool Size values", Assert.Single(sizeException.Args));

        var timeouts = new SnowflakeDbConnectionPool.PartitionRegistry();
        timeouts.GetOrCreate(
            PoolKeyFactory.From("account=e1"),
            ConnectionPoolConfig.Default with { ExpirationTimeout = TimeSpan.FromSeconds(60) });
        timeouts.GetOrCreate(
            PoolKeyFactory.From("account=e2"),
            ConnectionPoolConfig.Default with { ExpirationTimeout = TimeSpan.FromSeconds(61) });

        var timeoutException = Assert.Throws<SnowflakeDriverException>(() => timeouts.UniformTimeoutSeconds());
        Assert.Equal(SFError.INCONSISTENT_RESULT_ERROR, timeoutException.Error);
        Assert.Equal("Multiple pools have different Timeout values", Assert.Single(timeoutException.Args));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ClearAllDetachesThePartitionAndClearDoesNot()
    {
        var registry = new SnowflakeDbConnectionPool.PartitionRegistry();
        var key = PoolKeyFactory.From("account=clear");
        var first = registry.GetOrCreate(key);
        first.Clear();
        Assert.Same(first, registry.GetOrCreate(key));

        registry.ClearAll();
        var second = registry.GetOrCreate(key);

        Assert.NotSame(first, second);
        Assert.Equal(0, registry.CurrentPoolSize());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ConcurrentLookupOfOneKeyCreatesOnePartition()
    {
        var registry = new SnowflakeDbConnectionPool.PartitionRegistry();
        var key = PoolKeyFactory.From("account=concurrent");
        const int count = 10;
        var found = new SnowflakeDbConnectionPool[count];
        using var start = new Barrier(count + 1);
        var threads = new Thread[count];
        for (var index = 0; index < count; index++)
        {
            var captured = index;
            threads[captured] = new Thread(() =>
            {
                start.SignalAndWait();
                found[captured] = registry.GetOrCreate(key);
            });
            threads[captured].Start();
        }

        start.SignalAndWait();
        foreach (var thread in threads)
            thread.Join();

        foreach (var partition in found)
            Assert.Same(found[0], partition);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ToStringOmitsTheConnectionStringAndSecrets()
    {
        using var secret = TestSecureString.From("secure-value");
        var key = PoolKeyFactory.From("password=embedded-secret", secret);

        Assert.Equal(nameof(PoolKey), key.ToString());
        Assert.DoesNotContain("embedded-secret", key.ToString(), StringComparison.Ordinal);
        Assert.DoesNotContain("secure-value", key.ToString(), StringComparison.Ordinal);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void NullConnectionStringThrowsArgumentNullException()
    {
        var exception = Assert.Throws<ArgumentNullException>(() => PoolKeyFactory.From(null!));

        Assert.Equal("connectionString", exception.ParamName);
    }
}
