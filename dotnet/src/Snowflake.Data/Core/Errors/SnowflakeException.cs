using Snowflake.Data.Interfaces;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Core.Errors;

/// <summary>
/// Internal exception for failures raised below the ADO.NET boundary. Public ADO.NET members catch it
/// and throw <see cref="ToPublicException"/> instead; a exception never reaches the caller.
/// </summary>
internal abstract class SnowflakeException : Exception, ISnowflakeException
{
    private protected SnowflakeException(string message, string? queryId, Exception? innerException)
        : base(message, innerException)
    {
        QueryId = queryId;
    }

    /// <summary>Snowflake query ID the failure belongs to, when known.</summary>
    internal string? QueryId { get; set; }

    public abstract Exception ToPublicException();

    /// <summary>Maps an error sf_core reported onto a carrier.</summary>
    /// <remarks>
    /// The carrier copies the fields it exposes and does not keep <paramref name="exception"/> as its inner
    /// exception: the message of a <see cref="ServiceException"/> is the whole <see cref="DriverException"/>,
    /// including the error trace, root cause, request ID and parameter values, none of which may reach the
    /// public exception or the logs.
    /// </remarks>
    internal static SnowflakeException FromCore(ServiceException exception)
    {
        var error = exception.Error;
        var queryId = error.HasQueryId ? error.QueryId : null;

        // TODO(SNOW-4382591): map ErrorKind and the remaining DriverException fields onto the final exception contract.
        if (error.HasVendorCode)
            return new SnowflakeServerException(error.HasSqlState ? error.SqlState : null, error.VendorCode, error.Message, queryId);

        return new SnowflakeDriverException(SFError.INTERNAL_ERROR, queryId, error.Message);
    }

    internal static SnowflakeException FromCore(TransportException exception) =>
        new SnowflakeDriverException(SFError.INTERNAL_ERROR, null, exception, exception.Message);
}
