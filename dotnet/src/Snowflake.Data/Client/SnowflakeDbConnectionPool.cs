using System.Collections.Concurrent;
using System.Security;
using Snowflake.Data.Compatibility;
using Snowflake.Data.Core;
using Snowflake.Data.Core.Errors;
using Snowflake.Data.Core.Session;

namespace Snowflake.Data.Client;

/// <summary>
/// Partitions of the connection pool and the static API that addresses them.
/// </summary>
/// <remarks>
/// An instance is one partition, created by a <see cref="PartitionRegistry"/>.
/// The static members delegate to <see cref="Default"/>.
/// <see cref="GetPool"/> and <see cref="SnowflakeDbSessionPool"/> remain for callers of the legacy
/// connector and are obsolete. A null connection string throws <see cref="ArgumentNullException"/>.
/// A new partition reads its pool settings from the connection string. Opening a connection does not
/// take a session from a partition; acquiring and releasing sessions is planned.
/// </remarks>
public class SnowflakeDbConnectionPool : ISnowflakeDbConnectionPoolPartition
{
    /// <summary>The obsolete message on <see cref="GetPool"/> and <see cref="SnowflakeDbSessionPool"/>.</summary>
    internal const string SessionPoolObsoleteMessage =
        $"Use {nameof(GetPoolPartition)} and its properties. SnowflakeDbSessionPool will be removed in the next major version.";

    private const string GlobalSetterMessage =
        "You cannot change connection pool parameters for all the pools. Instead you can change it on a particular pool";

    private static readonly PartitionRegistry s_default = new();

    private readonly PoolKey _key;
    private readonly ConnectionPoolConfig _config;

    /// <summary>Creates a partition for <paramref name="key"/> with <paramref name="config"/>.</summary>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="config"/> is null.</exception>
    internal SnowflakeDbConnectionPool(PoolKey key, ConnectionPoolConfig config)
    {
        ArgumentNullExceptionHelper.ThrowIfNull(config, nameof(config));
        _key = key;
        _config = config;
    }

    /// <summary>Configured maximum number of sessions.</summary>
    public int MaxPoolSize => _config.MaxPoolSize;

    /// <summary>Configured minimum number of sessions.</summary>
    public int MinPoolSize => _config.MinPoolSize;

    /// <summary>Number of sessions this partition holds. No sessions are held.</summary>
    public int CurrentPoolSize => 0;

    /// <summary>Whether this partition pools sessions.</summary>
    public bool PoolingEnabled => _config.PoolingEnabled;

    /// <summary>How long a session may stay in this partition.</summary>
    public TimeSpan ExpirationTimeout => _config.ExpirationTimeout;

    /// <summary>How long opening a session may take.</summary>
    public TimeSpan ConnectionTimeout => _config.ConnectionTimeout;

    /// <summary>How long a request waits for an idle session.</summary>
    public TimeSpan WaitingForIdleSessionTimeout => _config.WaitingForIdleSessionTimeout;

    /// <summary>How a session is treated after its session context changes.</summary>
    public ChangedSessionBehavior ChangedSession => _config.ChangedSession;

    /// <summary>The registry the static members delegate to.</summary>
    internal static PartitionRegistry Default => s_default;

    /// <summary>The key that selects this partition.</summary>
    internal PoolKey Key => _key;

    /// <summary>Clears every partition in <see cref="Default"/>.</summary>
    public static void ClearAllPools() => s_default.ClearAll();

    /// <summary>Always throws <see cref="Exception"/>; pool sizes are set per pool.</summary>
    /// <exception cref="Exception">Thrown with the legacy message. Pool sizes are set per pool.</exception>
    public static void SetMaxPoolSize(int maxPoolSize) =>
        throw new Exception(GlobalSetterMessage);

    /// <summary>
    /// Returns the maximum pool size shared by every partition, or the default when none exist.
    /// </summary>
    /// <remarks>Differing maximum sizes throw an exception whose type is not yet defined.</remarks>
    public static int GetMaxPoolSize()
    {
        try
        {
            return s_default.UniformMaxPoolSize();
        }
        catch (SnowflakeDriverException carrier)
        {
            throw carrier.ToPublicException();
        }
    }

    /// <summary>Always throws <see cref="Exception"/>; the expiration timeout is set per pool.</summary>
    /// <exception cref="Exception">Thrown with the legacy message. The expiration timeout is set per pool.</exception>
    public static void SetTimeout(long connectionTimeout) =>
        throw new Exception(GlobalSetterMessage);

    /// <summary>
    /// Returns the session expiration timeout shared by every partition, in seconds, or the default when none exist.
    /// </summary>
    /// <remarks>Differing expiration timeouts throw an exception whose type is not yet defined.</remarks>
    public static long GetTimeout()
    {
        try
        {
            return s_default.UniformTimeoutSeconds();
        }
        catch (SnowflakeDriverException carrier)
        {
            throw carrier.ToPublicException();
        }
    }

    /// <summary>Returns the number of sessions across all partitions.</summary>
    public static int GetCurrentPoolSize() => s_default.CurrentPoolSize();

    /// <summary>Always throws <see cref="Exception"/>; pooling is set per pool.</summary>
    /// <exception cref="Exception">Thrown with the legacy message. Pooling is set per pool.</exception>
    public static bool SetPooling(bool isEnable) =>
        throw new Exception(GlobalSetterMessage);

    /// <summary>Returns whether pooling is enabled. This getter always returns true and does not read a partition.</summary>
    public static bool GetPooling() => true;

    // TODO(SNOW-4628635): both GetPool overloads are obsolete so that the pool API converges on
    // partitions (GetPoolPartition and the instance properties). Migrating may cost callers more than
    // the naming gains and slow adoption. The private and public preview release review weighs it with
    // the other behavior differences: if only a few remain, the obsolescence is lifted; otherwise it
    // stays until a breaking-change release removes both overloads.
    /// <summary>
    /// Returns the legacy pool view of the partition for <paramref name="connectionString"/>.
    /// </summary>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="connectionString"/> is null.</exception>
    [Obsolete(SessionPoolObsoleteMessage)]
    public static SnowflakeDbSessionPool GetPool(string connectionString) =>
        new(GetPoolPartition(connectionString));

    /// <summary>
    /// Returns the legacy pool view of the partition for <paramref name="connectionString"/> and the secrets.
    /// </summary>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="connectionString"/> is null.</exception>
    /// <exception cref="ArgumentOutOfRangeException">
    /// Thrown when a secret is longer than <c>int.MaxValue / 2</c> characters.
    /// </exception>
    [Obsolete(SessionPoolObsoleteMessage)]
    public static SnowflakeDbSessionPool GetPool(
        string connectionString,
        SecureString? password,
        SecureString? oauthClientSecret = null,
        SecureString? token = null) =>
        new(GetPoolPartition(connectionString, password, oauthClientSecret, token));

    /// <summary>Returns the partition for <paramref name="connectionString"/>.</summary>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="connectionString"/> is null.</exception>
    public static SnowflakeDbConnectionPool GetPoolPartition(string connectionString) =>
        GetPoolPartition(connectionString, null, null, null);

    /// <summary>
    /// Returns the partition for <paramref name="connectionString"/> and the secrets.
    /// A null or empty secret is absent.
    /// </summary>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="connectionString"/> is null.</exception>
    /// <exception cref="ArgumentOutOfRangeException">
    /// Thrown when a secret is longer than <c>int.MaxValue / 2</c> characters.
    /// </exception>
    /// <remarks>An invalid pool setting in <paramref name="connectionString"/> throws an exception whose type is not yet defined.</remarks>
    public static SnowflakeDbConnectionPool GetPoolPartition(
        string connectionString,
        SecureString? password,
        SecureString? oauthClientSecret = null,
        SecureString? token = null)
    {
        var key = PoolKeyFactory.From(connectionString, password, oauthClientSecret, token);
        try
        {
            return s_default.GetOrCreate(key);
        }
        catch (SnowflakeDriverException carrier)
        {
            throw carrier.ToPublicException();
        }
    }

    /// <summary>Returns whether pooling is enabled for the partition of <paramref name="connectionString"/>.</summary>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="connectionString"/> is null.</exception>
    public static bool GetPooling(
        string connectionString,
        SecureString? password = null,
        SecureString? oauthClientSecret = null,
        SecureString? token = null) =>
        GetPoolPartition(connectionString, password, oauthClientSecret, token).PoolingEnabled;

    /// <summary>Returns the minimum size of the partition for <paramref name="connectionString"/>.</summary>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="connectionString"/> is null.</exception>
    public static int GetMinPoolSize(
        string connectionString,
        SecureString? password = null,
        SecureString? oauthClientSecret = null,
        SecureString? token = null) =>
        GetPoolPartition(connectionString, password, oauthClientSecret, token).MinPoolSize;

    /// <summary>Returns the maximum size of the partition for <paramref name="connectionString"/>.</summary>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="connectionString"/> is null.</exception>
    public static int GetMaxPoolSize(
        string connectionString,
        SecureString? password = null,
        SecureString? oauthClientSecret = null,
        SecureString? token = null) =>
        GetPoolPartition(connectionString, password, oauthClientSecret, token).MaxPoolSize;

    /// <summary>Returns the number of sessions in the partition for <paramref name="connectionString"/>.</summary>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="connectionString"/> is null.</exception>
    public static int GetCurrentPoolSize(
        string connectionString,
        SecureString? password = null,
        SecureString? oauthClientSecret = null,
        SecureString? token = null) =>
        GetPoolPartition(connectionString, password, oauthClientSecret, token).CurrentPoolSize;

    /// <summary>
    /// Returns the session expiration timeout of the partition for <paramref name="connectionString"/>, in seconds.
    /// </summary>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="connectionString"/> is null.</exception>
    public static long GetExpirationTimeout(
        string connectionString,
        SecureString? password = null,
        SecureString? oauthClientSecret = null,
        SecureString? token = null) =>
        PoolTimeout.ToWholeSeconds(
            GetPoolPartition(connectionString, password, oauthClientSecret, token).ExpirationTimeout,
            mapInfiniteToMinusOne: true);

    /// <summary>
    /// Returns the connection timeout of the partition for <paramref name="connectionString"/>, in seconds.
    /// </summary>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="connectionString"/> is null.</exception>
    public static long GetConnectionTimeout(
        string connectionString,
        SecureString? password = null,
        SecureString? oauthClientSecret = null,
        SecureString? token = null) =>
        PoolTimeout.ToWholeSeconds(
            GetPoolPartition(connectionString, password, oauthClientSecret, token).ConnectionTimeout,
            mapInfiniteToMinusOne: true);

    /// <summary>
    /// Returns how long the partition for <paramref name="connectionString"/> waits for an idle session, in seconds.
    /// </summary>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="connectionString"/> is null.</exception>
    public static long GetWaitForIdleSessionTimeout(
        string connectionString,
        SecureString? password = null,
        SecureString? oauthClientSecret = null,
        SecureString? token = null) =>
        PoolTimeout.ToWholeSeconds(
            GetPoolPartition(connectionString, password, oauthClientSecret, token).WaitingForIdleSessionTimeout,
            mapInfiniteToMinusOne: false);

    /// <summary>Clears the partition for <paramref name="connectionString"/>.</summary>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="connectionString"/> is null.</exception>
    public static void ClearPool(
        string connectionString,
        SecureString? password = null,
        SecureString? oauthClientSecret = null,
        SecureString? token = null) =>
        GetPoolPartition(connectionString, password, oauthClientSecret, token).Clear();

    /// <summary>
    /// Returns what the partition for <paramref name="connectionString"/> does with a session whose context changed.
    /// </summary>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="connectionString"/> is null.</exception>
    public static ChangedSessionBehavior GetChangedSession(
        string connectionString,
        SecureString? password = null,
        SecureString? oauthClientSecret = null,
        SecureString? token = null) =>
        GetPoolPartition(connectionString, password, oauthClientSecret, token).ChangedSession;

    /// <summary>Releases every session this partition holds. The partition holds none.</summary>
    public void Clear()
    {
    }

    /// <summary>Maps pool keys to partitions and answers the questions that span every partition.</summary>
    internal sealed class PartitionRegistry
    {
        private const string InconsistentMaxPoolSizeMessage =
            "Multiple pools have different Max Pool Size values";

        private const string InconsistentTimeoutMessage =
            "Multiple pools have different Timeout values";

        private readonly ConcurrentDictionary<PoolKey, SnowflakeDbConnectionPool> _partitions = new();

        /// <summary>Creates an empty registry.</summary>
        internal PartitionRegistry()
        {
        }

        /// <summary>Returns the number of keys in this registry.</summary>
        internal int RegisteredKeyCount => _partitions.Count;

        /// <summary>
        /// Returns the partition for <paramref name="key"/>.
        /// The connection string is parsed only when the key is new, and a failed parse registers nothing.
        /// Concurrent first lookups of one key may each parse; one partition is kept.
        /// </summary>
        /// <exception cref="SnowflakeDriverException">A pool setting in the connection string is invalid.</exception>
        internal SnowflakeDbConnectionPool GetOrCreate(PoolKey key) =>
            GetOrCreate(key, () => ConnectionPoolConfigParser.Parse(key.ConnectionString));

        /// <summary>
        /// Returns the partition for <paramref name="key"/>. A key that is already registered keeps
        /// its partition; <paramref name="config"/> applies only when the key is new.
        /// </summary>
        /// <exception cref="ArgumentNullException">Thrown when <paramref name="config"/> is null.</exception>
        internal SnowflakeDbConnectionPool GetOrCreate(PoolKey key, ConnectionPoolConfig config)
        {
            ArgumentNullExceptionHelper.ThrowIfNull(config, nameof(config));
            return GetOrCreate(key, () => config);
        }

        /// <summary>
        /// Returns the partition for <paramref name="key"/>.
        /// <paramref name="configFactory"/> runs only when the key is new, without holding a lock; a factory that
        /// throws registers nothing. Concurrent first lookups of one key may each run the factory; one partition is kept.
        /// </summary>
        internal SnowflakeDbConnectionPool GetOrCreate(PoolKey key, Func<ConnectionPoolConfig> configFactory)
        {
            ArgumentNullExceptionHelper.ThrowIfNull(configFactory, nameof(configFactory));
            if (_partitions.TryGetValue(key, out var partition))
                return partition;

            return _partitions.GetOrAdd(key, new SnowflakeDbConnectionPool(key, configFactory()));
        }

        /// <summary>Returns the number of sessions across the partitions in this registry.</summary>
        internal int CurrentPoolSize() => _partitions.Values.Sum(partition => partition.CurrentPoolSize);

        /// <summary>
        /// Returns the shared maximum pool size, or the default when this registry is empty.
        /// </summary>
        /// <exception cref="SnowflakeDriverException">Partitions report different maximum sizes.</exception>
        internal int UniformMaxPoolSize()
        {
            var sizes = _partitions.Values.Select(partition => partition.MaxPoolSize).Distinct().ToArray();
            return sizes.Length switch
            {
                0 => ConnectionPoolConfig.Default.MaxPoolSize,
                1 => sizes[0],
                _ => throw new SnowflakeDriverException(
                    SFError.INCONSISTENT_RESULT_ERROR,
                    null,
                    InconsistentMaxPoolSizeMessage),
            };
        }

        /// <summary>
        /// Returns the shared expiration timeout in whole seconds, or the default when this registry is empty.
        /// </summary>
        /// <exception cref="SnowflakeDriverException">Partitions report different expiration timeouts.</exception>
        internal long UniformTimeoutSeconds()
        {
            var timeouts = _partitions.Values
                .Select(partition => PoolTimeout.ToWholeSeconds(partition.ExpirationTimeout, mapInfiniteToMinusOne: true))
                .Distinct()
                .ToArray();
            return timeouts.Length switch
            {
                0 => PoolTimeout.ToWholeSeconds(ConnectionPoolConfig.Default.ExpirationTimeout, mapInfiniteToMinusOne: true),
                1 => timeouts[0],
                _ => throw new SnowflakeDriverException(
                    SFError.INCONSISTENT_RESULT_ERROR,
                    null,
                    InconsistentTimeoutMessage),
            };
        }

        /// <summary>
        /// Removes every partition and then clears each one. A lookup that runs concurrently registers a new partition.
        /// </summary>
        internal void ClearAll()
        {
            var removed = new List<SnowflakeDbConnectionPool>();
            foreach (var key in _partitions.Keys)
            {
                if (_partitions.TryRemove(key, out var partition))
                    removed.Add(partition);
            }

            foreach (var partition in removed)
                partition.Clear();
        }
    }
}
