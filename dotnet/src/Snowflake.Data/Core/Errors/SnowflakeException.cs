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
    /// including the error trace, request ID and parameter values, none of which may reach the public exception
    /// or the logs. The root cause is the deepest error in the sf_core chain and is often the only one that says
    /// what failed, so the carrier's message ends with it unless the message already contains it.
    /// </remarks>
    internal static SnowflakeException FromCore(ServiceException exception)
    {
        var error = exception.Error;
        var queryId = error.HasQueryId ? error.QueryId : null;
        var message = MessageWithRootCause(error);

        // TODO(SNOW-4382591): map ErrorKind and the remaining DriverException fields onto the final exception contract.
        if (error.HasVendorCode)
            return new SnowflakeServerException(error.HasSqlState ? error.SqlState : null, error.VendorCode, message, queryId);

        return new SnowflakeDriverException(SFError.INTERNAL_ERROR, queryId, message);
    }

    internal static SnowflakeException FromCore(TransportException exception) =>
        new SnowflakeDriverException(SFError.INTERNAL_ERROR, null, exception, exception.Message);

    private static string MessageWithRootCause(DriverException error)
    {
        if (!error.HasRootCause || string.IsNullOrEmpty(error.RootCause) || error.Message.Contains(error.RootCause))
            return error.Message;

        return string.IsNullOrEmpty(error.Message) ? error.RootCause : $"{error.Message.TrimEnd('.')}. {error.RootCause}";
    }
}
