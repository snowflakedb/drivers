using System.Runtime.InteropServices;
using System.Security;
using System.Security.Cryptography;
using Snowflake.Data.Compatibility;

namespace Snowflake.Data.Core.Session;

/// <summary>Builds a <see cref="PoolKey"/> from a connection string and the secrets that are not in it.</summary>
/// <remarks>
/// The digest is HMAC-SHA256, keyed by a per-process random key, over password, oauth client secret,
/// and token, in that order. Each secret is prefixed with its UTF-16 byte length. The slot order
/// and the length prefix mean the three slots cannot be confused.
/// A null secret and an empty secret are both absent. When all three are absent the digest is null,
/// so a call that omits them selects the same partition as a call that passes null or empty secrets.
/// </remarks>
internal static class PoolKeyFactory
{
    private const int MaxSecretCharacters = int.MaxValue / 2;

    private static readonly byte[] HmacKey = CreateKey();

    /// <summary>Builds the key for <paramref name="connectionString"/> and the three secrets.</summary>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="connectionString"/> is null.</exception>
    /// <exception cref="ArgumentOutOfRangeException">
    /// Thrown when a secret is longer than <c>int.MaxValue / 2</c> characters.
    /// </exception>
    internal static PoolKey From(
        string connectionString,
        SecureString? password = null,
        SecureString? oauthClientSecret = null,
        SecureString? token = null)
    {
        ArgumentNullExceptionHelper.ThrowIfNull(connectionString, nameof(connectionString));
        return new PoolKey(connectionString, DigestOrNull(password, oauthClientSecret, token));
    }

    private static string? DigestOrNull(SecureString? password, SecureString? oauthClientSecret, SecureString? token)
    {
        if (!HasSecret(password) && !HasSecret(oauthClientSecret) && !HasSecret(token))
            return null;

        using var hmac = new HMACSHA256(HmacKey);
        AppendSecret(hmac, password, nameof(password));
        AppendSecret(hmac, oauthClientSecret, nameof(oauthClientSecret));
        AppendSecret(hmac, token, nameof(token));
        hmac.TransformFinalBlock([], 0, 0);
        var hash = hmac.Hash ?? throw new InvalidOperationException("HMAC-SHA256 did not produce a digest.");
        return Convert.ToBase64String(hash);
    }

    private static void AppendSecret(HMAC hmac, SecureString? secret, string paramName)
    {
        var prefix = new byte[4];
        if (secret is not { Length: > 0 })
        {
            hmac.TransformBlock(prefix, 0, prefix.Length, prefix, 0);
            return;
        }

        if (secret.Length > MaxSecretCharacters)
        {
            throw new ArgumentOutOfRangeException(
                paramName,
                $"A pool secret longer than {MaxSecretCharacters} characters is rejected.");
        }

        var byteCount = secret.Length * 2;
        WriteBigEndianLength(prefix, byteCount);
        var buffer = new byte[byteCount];
        var handle = GCHandle.Alloc(buffer, GCHandleType.Pinned);
        var native = nint.Zero;
        try
        {
            native = Marshal.SecureStringToGlobalAllocUnicode(secret);
            Marshal.Copy(native, buffer, 0, byteCount);
            hmac.TransformBlock(prefix, 0, prefix.Length, prefix, 0);
            hmac.TransformBlock(buffer, 0, buffer.Length, buffer, 0);
        }
        finally
        {
            Array.Clear(buffer, 0, buffer.Length);
            if (handle.IsAllocated)
                handle.Free();
            if (native != nint.Zero)
                Marshal.ZeroFreeGlobalAllocUnicode(native);
        }
    }

    private static void WriteBigEndianLength(byte[] prefix, int byteCount)
    {
        prefix[0] = (byte)(byteCount >> 24);
        prefix[1] = (byte)(byteCount >> 16);
        prefix[2] = (byte)(byteCount >> 8);
        prefix[3] = (byte)byteCount;
    }

    private static bool HasSecret(SecureString? secret) => secret is { Length: > 0 };

    private static byte[] CreateKey()
    {
        var key = new byte[32];
        using var generator = RandomNumberGenerator.Create();
        generator.GetBytes(key);
        return key;
    }
}
