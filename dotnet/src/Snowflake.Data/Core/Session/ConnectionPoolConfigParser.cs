using System.Globalization;
using System.Text.RegularExpressions;
using Microsoft.Extensions.Logging;
using Snowflake.Data;
using Snowflake.Data.Core;
using Snowflake.Data.Core.Errors;

namespace Snowflake.Data.Core.Session;

/// <summary>
/// Reads one <see cref="ConnectionPoolConfig"/> from a connection string.
/// </summary>
/// <remarks>
/// Keys are case-insensitive. A missing key uses the default. An empty value uses the default.
/// <c>ms</c>, <c>s</c>, and <c>m</c> name the timeout unit, and a timeout with no unit is seconds.
/// A negative expiration, connection timeout, or retry timeout means the timeout never elapses.
/// A positive retry timeout below 300 seconds is raised to 300 seconds. A finite connection timeout
/// is lowered to that retry timeout when the retry timeout is smaller. A zero retry timeout does not
/// cap the connection timeout. A negative wait for an idle session is rejected. <see cref="PoolingEnabled"/> defaults to
/// <see langword="false"/> for <c>externalbrowser</c>, <c>oauth_authorization_code</c>, and
/// <c>snowflake_jwt</c> with <c>PRIVATE_KEY_FILE</c> and no <c>PRIVATE_KEY_PWD</c> when the key is absent.
/// </remarks>
internal static class ConnectionPoolConfigParser
{
    internal const string MaxPoolSizeKey = "MAXPOOLSIZE";
    internal const string MinPoolSizeKey = "MINPOOLSIZE";
    internal const string ChangedSessionKey = "CHANGEDSESSION";
    internal const string WaitingForIdleSessionTimeoutKey = "WAITINGFORIDLESESSIONTIMEOUT";
    internal const string ExpirationTimeoutKey = "EXPIRATIONTIMEOUT";
    internal const string PoolingEnabledKey = "POOLINGENABLED";
    internal const string ConnectionTimeoutKey = "CONNECTION_TIMEOUT";

    private const string AuthenticatorKey = "AUTHENTICATOR";
    private const string PrivateKeyFileKey = "PRIVATE_KEY_FILE";
    private const string PrivateKeyPasswordKey = "PRIVATE_KEY_PWD";
    private const string UserKey = "USER";
    private const string RetryTimeoutKey = "RETRY_TIMEOUT";

    private const string ExternalBrowserAuthenticator = "externalbrowser";
    private const string KeyPairAuthenticator = "snowflake_jwt";
    private const string OAuthAuthorizationCodeAuthenticator = "oauth_authorization_code";

    private const string EmptyParameterMessage = "Parameter {Key} is empty. Using the default value.";
    private const string DisablePoolingMessage = "Disabling connection pooling for {Authentication} authentication";
    private const string PoolingNotRecommendedMessage =
        "Connection pooling is enabled for {Authentication} authentication which is not recommended";
    private const string ZeroConnectionTimeoutMessage =
        "Connection timeout provided is 0. Timeout will be infinite";
    private const string NegativeConnectionTimeoutMessage =
        "Connection timeout provided is negative. Timeout will be infinite.";
    private const string RetryTimeoutBelowMinimumMessage =
        "Max retry timeout provided is less than the allowed minimum value of 00:05:00";
    private const string ZeroRetryTimeoutMessage =
        "Max retry timeout provided is 0. Timeout will be infinite";
    private const string NegativeRetryTimeoutMessage =
        "Max retry timeout provided is negative. Timeout will be infinite";
    private const string ConnectionTimeoutAboveRetryMessage =
        "Connection timeout greater than retry timeout. Setting connection time same as retry timeout";
    private const string OAuthPoolingWithoutUserMessage =
        "You cannot enable pooling for oauth authorization code authentication without specifying a user in the connection string.";
    private const string ZeroIdleWaitMessage =
        "Waiting for idle session timeout is 0. There will be no waiting for idle session.";

    private static readonly TimeSpan MinimumRetryTimeout = TimeSpan.FromSeconds(300);

    private static readonly Regex s_timeoutFormat = new(@"^(-)?[0-9]{1,10}[mM]?[sS]?$", RegexOptions.CultureInvariant);

    private static readonly ILogger s_logger = LoggerAndTelemetryStubs.GetLogger<LogCategory>();

    /// <summary>Reads the pool settings in <paramref name="connectionString"/>.</summary>
    /// <exception cref="SnowflakeDriverException">A pool setting is invalid, or the connection string cannot be parsed.</exception>
    internal static ConnectionPoolConfig Parse(string connectionString)
    {
        ConnectionStringParser parser;
        try
        {
            parser = new ConnectionStringParser(connectionString);
        }
        catch (ArgumentException exception)
        {
            throw new SnowflakeDriverException(
                SFError.INVALID_CONNECTION_STRING,
                null,
                exception,
                $"The connection string could not be parsed: {exception.Message}");
        }

        var poolingEnabled = ReadPoolingEnabled(parser);
        var maxPoolSize = ReadPositiveInt(parser, MaxPoolSizeKey, ConnectionPoolConfig.Default.MaxPoolSize);
        var minPoolSize = ReadNonNegativeInt(parser, MinPoolSizeKey, ConnectionPoolConfig.Default.MinPoolSize);
        if (minPoolSize > maxPoolSize)
        {
            throw new SnowflakeDriverException(
                SFError.INVALID_CONNECTION_STRING,
                null,
                "MinPoolSize cannot be greater than MaxPoolSize.");
        }

        var waiting = ReadTimeout(parser, WaitingForIdleSessionTimeoutKey, ConnectionPoolConfig.Default.WaitingForIdleSessionTimeout);
        if (waiting == Timeout.InfiniteTimeSpan)
        {
            throw new SnowflakeDriverException(
                SFError.INVALID_CONNECTION_STRING,
                null,
                $"{WaitingForIdleSessionTimeoutKey} cannot be infinite.");
        }

        if (waiting == TimeSpan.Zero)
            s_logger.LogWarning(ZeroIdleWaitMessage);

        var connectionTimeout = ReadConnectionTimeout(parser);
        return new ConnectionPoolConfig
        {
            MaxPoolSize = maxPoolSize,
            MinPoolSize = minPoolSize,
            ChangedSession = ReadChangedSession(parser),
            WaitingForIdleSessionTimeout = waiting,
            ExpirationTimeout = ReadTimeout(parser, ExpirationTimeoutKey, ConnectionPoolConfig.Default.ExpirationTimeout),
            PoolingEnabled = poolingEnabled,
            ConnectionTimeout = connectionTimeout,
        };
    }

    private static int ReadPositiveInt(ConnectionStringParser parser, string key, int defaultValue) =>
        ReadOptional(parser, key) is { } raw ? ParseInt(key, raw, allowZero: false) : defaultValue;

    private static int ReadNonNegativeInt(ConnectionStringParser parser, string key, int defaultValue) =>
        ReadOptional(parser, key) is { } raw ? ParseInt(key, raw, allowZero: true) : defaultValue;

    private static ChangedSessionBehavior ReadChangedSession(ConnectionStringParser parser)
    {
        if (ReadOptional(parser, ChangedSessionKey) is not { } raw)
            return ConnectionPoolConfig.Default.ChangedSession;

        if (!Enum.TryParse(raw, ignoreCase: true, out ChangedSessionBehavior value) || !Enum.IsDefined(typeof(ChangedSessionBehavior), value))
            throw Invalid(ChangedSessionKey);

        return value;
    }

    private static TimeSpan ReadTimeout(ConnectionStringParser parser, string key, TimeSpan defaultValue) =>
        ReadOptional(parser, key) is { } raw ? ParseTimeout(key, raw) : defaultValue;

    private static TimeSpan ReadConnectionTimeout(ConnectionStringParser parser)
    {
        var connectionTimeout = ReadProvidedConnectionTimeout(parser);
        var retryTimeout = ReadRetryTimeout(parser);
        if (retryTimeout != Timeout.InfiniteTimeSpan && retryTimeout < connectionTimeout)
        {
            s_logger.LogWarning(ConnectionTimeoutAboveRetryMessage);
            return retryTimeout;
        }

        return connectionTimeout;
    }

    private static TimeSpan ReadProvidedConnectionTimeout(ConnectionStringParser parser)
    {
        if (ReadOptional(parser, ConnectionTimeoutKey) is not { } raw)
            return ConnectionPoolConfig.Default.ConnectionTimeout;

        var timeout = ParseTimeout(ConnectionTimeoutKey, raw);
        if (timeout == TimeSpan.Zero)
        {
            s_logger.LogWarning(ZeroConnectionTimeoutMessage);
            return Timeout.InfiniteTimeSpan;
        }

        if (timeout == Timeout.InfiniteTimeSpan)
            s_logger.LogWarning(NegativeConnectionTimeoutMessage);

        return timeout;
    }

    private static TimeSpan ReadRetryTimeout(ConnectionStringParser parser)
    {
        if (ReadOptional(parser, RetryTimeoutKey) is not { } raw)
            return MinimumRetryTimeout;

        var timeout = ParseTimeout(RetryTimeoutKey, raw);
        if (timeout > TimeSpan.Zero && timeout < MinimumRetryTimeout)
        {
            s_logger.LogWarning(RetryTimeoutBelowMinimumMessage);
            return MinimumRetryTimeout;
        }

        if (timeout == TimeSpan.Zero)
        {
            s_logger.LogWarning(ZeroRetryTimeoutMessage);
            return Timeout.InfiniteTimeSpan;
        }

        if (timeout == Timeout.InfiniteTimeSpan)
            s_logger.LogWarning(NegativeRetryTimeoutMessage);

        return timeout;
    }

    private static bool ReadPoolingEnabled(ConnectionStringParser parser)
    {
        var pooling = ReadOptional(parser, PoolingEnabledKey);
        var enabled = pooling is null
            ? ConnectionPoolConfig.Default.PoolingEnabled
            : ParseBool(PoolingEnabledKey, pooling);
        var provided = pooling is not null;

        var authenticator = ReadPresent(parser, AuthenticatorKey);
        if (IsAuthenticator(authenticator, OAuthAuthorizationCodeAuthenticator) && provided && enabled && ReadPresent(parser, UserKey) is null)
        {
            throw new SnowflakeDriverException(
                SFError.INVALID_CONNECTION_STRING,
                null,
                OAuthPoolingWithoutUserMessage);
        }

        var restricted = RestrictedAuthentication(parser, authenticator);
        if (restricted is null)
            return enabled;

        if (!provided && enabled)
        {
            s_logger.LogInformation(DisablePoolingMessage, restricted);
            return false;
        }

        if (provided && enabled)
            s_logger.LogWarning(PoolingNotRecommendedMessage, restricted);

        return enabled;
    }

    private static string? RestrictedAuthentication(ConnectionStringParser parser, string? authenticator)
    {
        if (IsAuthenticator(authenticator, ExternalBrowserAuthenticator))
            return "external browser";

        if (IsAuthenticator(authenticator, KeyPairAuthenticator)
            && ReadPresent(parser, PrivateKeyFileKey) is not null
            && ReadPresent(parser, PrivateKeyPasswordKey) is null)
            return "key pair with private key in a file";

        if (IsAuthenticator(authenticator, OAuthAuthorizationCodeAuthenticator))
            return "oauth authorization code";

        return null;
    }

    private static string? ReadOptional(ConnectionStringParser parser, string key)
    {
        if (!parser.TryGetValue(key, out var value))
            return null;

        if (string.IsNullOrEmpty(value))
        {
            s_logger.LogWarning(EmptyParameterMessage, key);
            return null;
        }

        return value;
    }

    private static string? ReadPresent(ConnectionStringParser parser, string key)
    {
        if (!parser.TryGetValue(key, out var value) || string.IsNullOrEmpty(value))
            return null;

        return value;
    }

    private static int ParseInt(string key, string raw, bool allowZero)
    {
        int value;
        try
        {
            value = int.Parse(raw, NumberStyles.Integer, CultureInfo.InvariantCulture);
        }
        catch (Exception exception) when (exception is FormatException or OverflowException)
        {
            throw Invalid(key, exception);
        }

        if (value < 0 || (!allowZero && value == 0))
            throw Invalid(key);

        return value;
    }

    private static bool ParseBool(string key, string raw)
    {
        try
        {
            return bool.Parse(raw);
        }
        catch (FormatException exception)
        {
            throw Invalid(key, exception);
        }
    }

    private static TimeSpan ParseTimeout(string key, string raw)
    {
        if (!s_timeoutFormat.IsMatch(raw))
            throw Invalid(key);

        var unitStart = raw.Length;
        while (unitStart > 0 && !char.IsDigit(raw[unitStart - 1]))
            unitStart--;

        int numeric;
        try
        {
            numeric = int.Parse(raw.Substring(0, unitStart), NumberStyles.Integer, CultureInfo.InvariantCulture);
        }
        catch (Exception exception) when (exception is FormatException or OverflowException)
        {
            throw Invalid(key, exception);
        }

        if (numeric < 0)
            return Timeout.InfiniteTimeSpan;

        var unit = raw.Substring(unitStart).ToLowerInvariant();
        return unit switch
        {
            "" or "s" => TimeSpan.FromSeconds(numeric),
            "ms" => TimeSpan.FromMilliseconds(numeric),
            "m" => TimeSpan.FromMinutes(numeric),
            _ => throw Invalid(key),
        };
    }

    private static bool IsAuthenticator(string? value, string name) =>
        name.Equals(value, StringComparison.OrdinalIgnoreCase);

    private static SnowflakeDriverException Invalid(string key, Exception? inner = null) =>
        new(SFError.INVALID_CONNECTION_STRING, null, inner, $"Invalid value of parameter {key}.");

    private sealed class LogCategory
    {
    }
}
