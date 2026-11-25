namespace Snowflake.Data.Core.Session;

/// <summary>Pool settings for one partition.</summary>
/// <remarks>
/// <see cref="Default"/> is what a new partition receives. The values are stored and not enforced;
/// enforcing them is planned. Connection-string keys are not read.
/// </remarks>
internal sealed record ConnectionPoolConfig
{
    /// <summary>Configured maximum number of sessions.</summary>
    internal int MaxPoolSize { get; init; } = 10;

    /// <summary>Configured minimum number of sessions.</summary>
    internal int MinPoolSize { get; init; } = 2;

    /// <summary>How a session is treated after its session context changes.</summary>
    internal ChangedSessionBehavior ChangedSession { get; init; } = ChangedSessionBehavior.Destroy;

    /// <summary>How long a request waits for an idle session.</summary>
    internal TimeSpan WaitingForIdleSessionTimeout { get; init; } = TimeSpan.FromSeconds(30);

    /// <summary>How long a session may stay in the partition.</summary>
    internal TimeSpan ExpirationTimeout { get; init; } = TimeSpan.FromHours(1);

    /// <summary>Whether the partition pools sessions.</summary>
    internal bool PoolingEnabled { get; init; } = true;

    /// <summary>How long opening a session may take.</summary>
    internal TimeSpan ConnectionTimeout { get; init; } = TimeSpan.FromSeconds(300);

    /// <summary>The configuration a new partition receives.</summary>
    internal static ConnectionPoolConfig Default { get; } = new();
}
