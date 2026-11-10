namespace Snowflake.Data.Core.Errors;

/// <summary>A failure reported with a Snowflake vendor code, as the server reports query errors.</summary>
internal sealed class SnowflakeServerException : SnowflakeException
{
    internal SnowflakeServerException(string? sqlState, int vendorCode, string message, string? queryId, Exception? innerException = null)
        : base(message, queryId, innerException)
    {
        SqlState = sqlState;
        VendorCode = vendorCode;
    }

    /// <summary>The ANSI SQL state the server reported, when it reported one.</summary>
    internal string? SqlState { get; }

    /// <summary>The Snowflake vendor code the server reported.</summary>
    internal int VendorCode { get; }

    public override Exception ToPublicException() =>
        new SnowflakeDbException(SqlState ?? string.Empty, VendorCode, Message, QueryId ?? string.Empty);
}
