namespace Snowflake.Data.Core.Errors;

/// <summary>A failure detected by the driver itself, identified by an <see cref="SFError"/>.</summary>
internal sealed class SnowflakeDriverException : SnowflakeException
{
    internal SnowflakeDriverException(SFError error, string? queryId, params object[] args)
        : this(error, queryId, null, args)
    {
    }

    internal SnowflakeDriverException(SFError error, string? queryId, Exception? innerException, params object[] args)
        : base(error.ToString(), queryId, innerException)
    {
        Error = error;
        Args = args;
    }

    internal SFError Error { get; }

    /// <summary>Values formatted into the message of <see cref="Error"/>.</summary>
    internal object[] Args { get; }

    public override Exception ToPublicException()
    {
        var exception = InnerException is null
            ? new SnowflakeDbException(Error, Args)
            : new SnowflakeDbException(InnerException, Error, Args);

        if (QueryId is not null)
            exception.QueryId = QueryId;

        return exception;
    }
}
