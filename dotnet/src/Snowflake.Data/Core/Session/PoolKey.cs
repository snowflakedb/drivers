using System.Diagnostics;

namespace Snowflake.Data.Core.Session;

/// <summary>
/// Identity of one pool partition: the connection string and a digest of the secrets that are not in it.
/// </summary>
/// <remarks>
/// <see cref="PoolKeyFactory"/> builds the digest. A null digest means all three secrets are absent.
/// Secret text is not stored on the key, and <see cref="ToString"/> does not include the connection
/// string or the digest.
/// </remarks>
internal readonly record struct PoolKey
{
    /// <summary>Creates a key for <paramref name="connectionString"/> and <paramref name="secretsDigest"/>.</summary>
    internal PoolKey(string connectionString, string? secretsDigest)
    {
        ConnectionString = connectionString;
        SecretsDigest = secretsDigest;
    }

    /// <summary>The connection string that identifies the partition.</summary>
    [DebuggerBrowsable(DebuggerBrowsableState.Never)]
    internal string ConnectionString { get; }

    /// <summary>Base64 HMAC-SHA256 of the secrets, or null when all three secrets are absent.</summary>
    internal string? SecretsDigest { get; }

    /// <summary>Returns the type name. The connection string and the digest are omitted.</summary>
    public override string ToString() => nameof(PoolKey);
}
