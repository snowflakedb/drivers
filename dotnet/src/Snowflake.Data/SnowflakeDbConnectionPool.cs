using System.Security;
using Snowflake.Data.Core.Session;
// ReSharper disable UnusedType.Global
// ReSharper disable UnusedMember.Global
// ReSharper disable UnusedParameter.Global

namespace Snowflake.Data;

/// <summary>
/// Entry point for the connection pools, matching the multiple-pool surface of the legacy .NET connector.
/// </summary>
/// <remarks>
/// The members are stubs. A member that takes a connection string addresses the pool for that
/// connection string; <c>password</c>, <c>oauthClientSecret</c>, and <c>token</c> supply the secrets
/// that are not in it. The setters for all pools throw <see cref="Exception"/> with the legacy
/// connector's message. The single-pool cache variant of the legacy connector is not part of this
/// surface.
/// </remarks>
public class SnowflakeDbConnectionPool
{
    private const string StubMessage =
        "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)";

    private const string GlobalSetterMessage =
        "You cannot change connection pool parameters for all the pools. Instead you can change it on a particular pool";

    /// <summary>Clears every pool.</summary>
    public static void ClearAllPools() => throw new NotImplementedException(StubMessage);

    /// <summary>Always throws <see cref="Exception"/>; pool sizes are set per pool.</summary>
    public static void SetMaxPoolSize(int maxPoolSize) =>
        throw new Exception(GlobalSetterMessage);

    /// <summary>Returns the maximum pool size.</summary>
    public static int GetMaxPoolSize() => throw new NotImplementedException(StubMessage);

    /// <summary>Always throws <see cref="Exception"/>; the expiration timeout is set per pool.</summary>
    public static void SetTimeout(long connectionTimeout) =>
        throw new Exception(GlobalSetterMessage);

    /// <summary>Returns the session expiration timeout in seconds.</summary>
    public static long GetTimeout() => throw new NotImplementedException(StubMessage);

    /// <summary>Returns the number of sessions across all pools.</summary>
    public static int GetCurrentPoolSize() => throw new NotImplementedException(StubMessage);

    /// <summary>Always throws <see cref="Exception"/>; pooling is set per pool.</summary>
    public static bool SetPooling(bool isEnable) =>
        throw new Exception(GlobalSetterMessage);

    /// <summary>Returns whether pooling is enabled globally.</summary>
    public static bool GetPooling() => throw new NotImplementedException(StubMessage);

    /// <summary>Returns whether pooling is enabled for the pool of <paramref name="connectionString"/>.</summary>
    public static bool GetPooling(
        string connectionString,
        SecureString? password = null,
        SecureString? oauthClientSecret = null,
        SecureString? token = null) =>
        throw new NotImplementedException(StubMessage);

    /// <summary>Returns the minimum size of the pool for <paramref name="connectionString"/>.</summary>
    public static int GetMinPoolSize(
        string connectionString,
        SecureString? password = null,
        SecureString? oauthClientSecret = null,
        SecureString? token = null) =>
        throw new NotImplementedException(StubMessage);

    /// <summary>Returns the maximum size of the pool for <paramref name="connectionString"/>.</summary>
    public static int GetMaxPoolSize(
        string connectionString,
        SecureString? password = null,
        SecureString? oauthClientSecret = null,
        SecureString? token = null) =>
        throw new NotImplementedException(StubMessage);

    /// <summary>Returns the number of sessions in the pool for <paramref name="connectionString"/>.</summary>
    public static int GetCurrentPoolSize(
        string connectionString,
        SecureString? password = null,
        SecureString? oauthClientSecret = null,
        SecureString? token = null) =>
        throw new NotImplementedException(StubMessage);

    /// <summary>
    /// Returns the session expiration timeout of the pool for <paramref name="connectionString"/>, in seconds.
    /// </summary>
    public static long GetExpirationTimeout(
        string connectionString,
        SecureString? password = null,
        SecureString? oauthClientSecret = null,
        SecureString? token = null) =>
        throw new NotImplementedException(StubMessage);

    /// <summary>
    /// Returns the connection timeout of the pool for <paramref name="connectionString"/>, in seconds.
    /// </summary>
    public static long GetConnectionTimeout(
        string connectionString,
        SecureString? password = null,
        SecureString? oauthClientSecret = null,
        SecureString? token = null) =>
        throw new NotImplementedException(StubMessage);

    /// <summary>
    /// Returns how long the pool for <paramref name="connectionString"/> waits for an idle session, in seconds.
    /// </summary>
    public static long GetWaitForIdleSessionTimeout(
        string connectionString,
        SecureString? password = null,
        SecureString? oauthClientSecret = null,
        SecureString? token = null) =>
        throw new NotImplementedException(StubMessage);

    /// <summary>Clears the pool for <paramref name="connectionString"/>.</summary>
    public static void ClearPool(
        string connectionString,
        SecureString? password = null,
        SecureString? oauthClientSecret = null,
        SecureString? token = null) =>
        throw new NotImplementedException(StubMessage);

    /// <summary>
    /// Returns what the pool for <paramref name="connectionString"/> does with a session whose context changed.
    /// </summary>
    public static ChangedSessionBehavior GetChangedSession(
        string connectionString,
        SecureString? password = null,
        SecureString? oauthClientSecret = null,
        SecureString? token = null) =>
        throw new NotImplementedException(StubMessage);
}
