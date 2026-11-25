using Snowflake.Data.Compatibility;
using Snowflake.Data.Core.Session;

namespace Snowflake.Data.Client;

// TODO(SNOW-4628635): this type is obsolete so that the pool API converges on partitions
// (GetPoolPartition and the SnowflakeDbConnectionPool properties). Migrating may cost callers more
// than the naming gains and slow adoption. The private and public preview release review weighs it
// with the other behavior differences: if only a few remain, the obsolescence is lifted; otherwise
// it stays until a breaking-change release removes the type.
/// <summary>
/// Legacy per-pool view of one <see cref="ISnowflakeDbConnectionPoolPartition"/>.
/// </summary>
/// <remarks>
/// The members forward to the partition. Timeouts are whole seconds.
/// Removing this type is planned for the next major version.
/// <see cref="SnowflakeDbConnectionPool.GetPoolPartition"/> returns the partition itself.
/// </remarks>
[Obsolete(SnowflakeDbConnectionPool.SessionPoolObsoleteMessage)]
public class SnowflakeDbSessionPool
{
    private readonly ISnowflakeDbConnectionPoolPartition _partition;

    /// <summary>Wraps <paramref name="partition"/>.</summary>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="partition"/> is null.</exception>
    internal SnowflakeDbSessionPool(ISnowflakeDbConnectionPoolPartition partition)
    {
        ArgumentNullExceptionHelper.ThrowIfNull(partition, nameof(partition));
        _partition = partition;
    }

    /// <summary>The partition whose values this facade returns.</summary>
    internal ISnowflakeDbConnectionPoolPartition Partition => _partition;

    /// <summary>Returns whether pooling is enabled for the partition.</summary>
    public bool GetPooling() => _partition.PoolingEnabled;

    /// <summary>Returns the minimum size of the partition.</summary>
    public int GetMinPoolSize() => _partition.MinPoolSize;

    /// <summary>Returns the maximum size of the partition.</summary>
    public int GetMaxPoolSize() => _partition.MaxPoolSize;

    /// <summary>Returns the number of sessions in the partition.</summary>
    public int GetCurrentPoolSize() => _partition.CurrentPoolSize;

    /// <summary>Returns the session expiration timeout of the partition, in seconds.</summary>
    public long GetExpirationTimeout() =>
        PoolTimeout.ToWholeSeconds(_partition.ExpirationTimeout, mapInfiniteToMinusOne: true);

    /// <summary>Returns the connection timeout of the partition, in seconds.</summary>
    public long GetConnectionTimeout() =>
        PoolTimeout.ToWholeSeconds(_partition.ConnectionTimeout, mapInfiniteToMinusOne: true);

    /// <summary>Returns how long the partition waits for an idle session, in seconds.</summary>
    public long GetWaitForIdleSessionTimeout() =>
        PoolTimeout.ToWholeSeconds(_partition.WaitingForIdleSessionTimeout, mapInfiniteToMinusOne: false);

    /// <summary>Clears the partition.</summary>
    public void ClearPool() => _partition.Clear();

    /// <summary>Returns what the partition does with a session whose context changed.</summary>
    public ChangedSessionBehavior GetChangedSession() => _partition.ChangedSession;
}
