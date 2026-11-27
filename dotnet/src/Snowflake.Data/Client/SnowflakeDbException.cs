using System.Data.Common;
using System.Globalization;
using Snowflake.Data.Core;
using Snowflake.Data.Core.Extensions;

namespace Snowflake.Data.Client;

/// <summary>A failure reported by Snowflake or detected by the driver.</summary>
/// <remarks>
/// <see cref="ErrorCode"/> is the Snowflake vendor code: a value from 270000 to 279999 for a failure the driver
/// detects, listed in <see cref="SFError"/>, and the server's code otherwise. <see cref="Exception.Message"/> has
/// the form <c>Error: {message} SqlState: {SqlState}, VendorCode: {ErrorCode}, QueryId: {QueryId}</c>.
/// </remarks>
public sealed partial class SnowflakeDbException : DbException
{
    private readonly string? _sqlState;

    /// <summary>Creates an exception for a failure that carries its own vendor code, such as a server error.</summary>
    /// <param name="sqlState">The ANSI SQL state of the failure.</param>
    /// <param name="vendorCode">The Snowflake vendor code of the failure.</param>
    /// <param name="errorMessage">The description of the failure.</param>
    /// <param name="queryId">The Snowflake query ID the failure belongs to.</param>
    public SnowflakeDbException(string sqlState, int vendorCode, string errorMessage, string queryId)
        : this(sqlState, vendorCode, errorMessage, queryId, null)
    {
    }

    /// <summary>Creates an exception for a driver error caused by <paramref name="innerException"/>.</summary>
    /// <param name="error">The driver error; its message is formatted with the message of <paramref name="innerException"/>.</param>
    /// <param name="queryId">The Snowflake query ID the failure belongs to.</param>
    /// <param name="innerException">The cause of the failure.</param>
    public SnowflakeDbException(SFError error, string queryId, Exception innerException)
        : this(null, error.GetErrorCode(), error.FormatMessage(innerException.Message), queryId, innerException)
    {
    }

    /// <summary>Creates an exception for a driver error.</summary>
    /// <param name="error">The driver error.</param>
    /// <param name="args">Values formatted into the message of <paramref name="error"/>.</param>
    public SnowflakeDbException(SFError error, params object[] args)
        : this(null, error.GetErrorCode(), error.FormatMessage(args), null, null)
    {
    }

    /// <summary>Creates an exception for a driver error with an ANSI SQL state.</summary>
    /// <param name="sqlState">The ANSI SQL state of the failure.</param>
    /// <param name="error">The driver error.</param>
    /// <param name="args">Values formatted into the message of <paramref name="error"/>.</param>
    public SnowflakeDbException(string sqlState, SFError error, params object[] args)
        : this(sqlState, error.GetErrorCode(), error.FormatMessage(args), null, null)
    {
    }

    /// <summary>Creates an exception for a driver error caused by <paramref name="innerException"/>.</summary>
    /// <param name="innerException">The cause of the failure.</param>
    /// <param name="error">The driver error.</param>
    /// <param name="args">Values formatted into the message of <paramref name="error"/>.</param>
    public SnowflakeDbException(Exception innerException, SFError error, params object[] args)
        : this(null, error.GetErrorCode(), error.FormatMessage(args), null, innerException)
    {
    }

    /// <summary>Creates an exception for a driver error with an ANSI SQL state, caused by <paramref name="innerException"/>.</summary>
    /// <param name="innerException">The cause of the failure.</param>
    /// <param name="sqlState">The ANSI SQL state of the failure.</param>
    /// <param name="error">The driver error.</param>
    /// <param name="args">Values formatted into the message of <paramref name="error"/>.</param>
    public SnowflakeDbException(Exception innerException, string sqlState, SFError error, params object[] args)
        : this(sqlState, error.GetErrorCode(), error.FormatMessage(args), null, innerException)
    {
    }

    /// <summary>Creates an exception from the parts an internal failure carries; any of them may be absent.</summary>
    /// <param name="sqlState">The ANSI SQL state of the failure, when known.</param>
    /// <param name="vendorCode">The Snowflake vendor code of the failure.</param>
    /// <param name="errorMessage">The description of the failure.</param>
    /// <param name="queryId">The Snowflake query ID the failure belongs to, when known.</param>
    /// <param name="innerException">The cause of the failure, when there is one.</param>
    internal SnowflakeDbException(string? sqlState, int vendorCode, string errorMessage, string? queryId, Exception? innerException)
        : base(FormatMessage(errorMessage, sqlState, vendorCode, queryId), innerException)
    {
        _sqlState = sqlState;
        ErrorCode = vendorCode;
        QueryId = queryId;
    }

    /// <summary>The Snowflake query ID the failure belongs to, or <see langword="null"/> when it is not known.</summary>
    public string? QueryId { get; set; }

    /// <summary>The Snowflake vendor code of the failure.</summary>
    public override int ErrorCode { get; }

    private static string FormatMessage(string errorMessage, string? sqlState, int vendorCode, string? queryId) =>
        string.Format(CultureInfo.InvariantCulture, "Error: {0} SqlState: {1}, VendorCode: {2}, QueryId: {3}", errorMessage, sqlState, vendorCode, queryId);
}
