namespace Snowflake.Data.Core.Session;

/// <summary>One connection-pool partition.</summary>
internal interface ISnowflakeDbConnectionPoolPartition
{
    /// <summary>Configured maximum number of sessions.</summary>
    int MaxPoolSize { get; }

    /// <summary>Configured minimum number of sessions.</summary>
    int MinPoolSize { get; }

    /// <summary>Number of sessions the partition holds.</summary>
    int CurrentPoolSize { get; }

    /// <summary>Whether the partition pools sessions.</summary>
    bool PoolingEnabled { get; }

    /// <summary>How long a session may stay in the partition.</summary>
    TimeSpan ExpirationTimeout { get; }

    /// <summary>How long opening a session may take.</summary>
    TimeSpan ConnectionTimeout { get; }

    /// <summary>How long a request waits for an idle session.</summary>
    TimeSpan WaitingForIdleSessionTimeout { get; }

    /// <summary>How a session is treated after its session context changes.</summary>
    ChangedSessionBehavior ChangedSession { get; }

    /// <summary>Releases every session this partition holds.</summary>
    void Clear();
}
