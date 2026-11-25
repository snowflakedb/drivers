using System.Data.Common;
using System.Reflection;

namespace Snowflake.Data.Tests.Assertions;

/// <summary>
/// Driver-agnostic assertions on database errors. Both the new and the legacy
/// <c>SnowflakeDbException</c> derive from <see cref="DbException"/>, so tests that run against
/// either driver assert on that type and its diagnostic fields.
/// </summary>
public static class DbExceptionAssert
{
    public static DbException Throws(
        Action action,
        string? sqlState = null,
        int? errorCode = null,
        string? messageContains = null)
    {
        var exception = Assert.ThrowsAny<DbException>(action);
        HasDiagnostics(exception, sqlState, errorCode, messageContains);
        return exception;
    }

    public static DbException Throws(
        Func<object?> func,
        string? sqlState = null,
        int? errorCode = null,
        string? messageContains = null)
    {
        var exception = Assert.ThrowsAny<DbException>(func);
        HasDiagnostics(exception, sqlState, errorCode, messageContains);
        return exception;
    }

    public static void HasDiagnostics(
        DbException exception,
        string? sqlState = null,
        int? errorCode = null,
        string? messageContains = null)
    {
        if (sqlState is not null)
            HasSqlState(exception, sqlState);
        if (errorCode is not null)
            HasErrorCode(exception, errorCode.Value);
        if (messageContains is not null)
            Assert.Contains(messageContains, exception.Message, StringComparison.Ordinal);
    }

    public static void HasSqlState(DbException exception, string expected) =>
        Assert.Equal(expected, GetSqlState(exception));

    public static void HasErrorCode(DbException exception, int expected) =>
        Assert.Equal(expected, exception.ErrorCode);

    /// <summary>
    /// Both drivers declare <c>SqlState</c> as a non-virtual property that hides
    /// <c>DbException.SqlState</c>, so reading it through the base type returns null.
    /// This reads the most-derived public <c>SqlState</c> declared on the runtime type.
    /// </summary>
    public static string? GetSqlState(DbException exception)
    {
        for (var type = exception.GetType(); type is not null; type = type.BaseType)
        {
            var property = type.GetProperty(
                "SqlState",
                BindingFlags.Public | BindingFlags.Instance | BindingFlags.DeclaredOnly);
            if (property is not null && property.PropertyType == typeof(string))
                return (string?)property.GetValue(exception);
        }

        return null;
    }
}
