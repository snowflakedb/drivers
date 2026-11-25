using System.Security;

namespace Snowflake.Data.Tests.Session;

/// <summary>Builds read-only secrets for pool-key tests.</summary>
internal static class TestSecureString
{
    /// <summary>A read-only <see cref="SecureString"/> holding <paramref name="value"/>.</summary>
    internal static SecureString From(string value)
    {
        var secret = new SecureString();
        foreach (var character in value)
            secret.AppendChar(character);
        secret.MakeReadOnly();
        return secret;
    }
}
