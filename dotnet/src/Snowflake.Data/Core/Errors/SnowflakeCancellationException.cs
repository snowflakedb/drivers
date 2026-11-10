namespace Snowflake.Data.Core.Errors;

/// <summary>
/// A cancelled operation that surfaces as <see cref="OperationCanceledException"/> wrapping a
/// <see cref="SnowflakeDbException"/> for backward compatibility.
/// </summary>
internal sealed class SnowflakeCancellationException : SnowflakeException
{
    internal SnowflakeCancellationException(SFError error, string? queryId, Exception? innerException, params object[] args)
        : base(error.ToString(), queryId, innerException)
    {
        Error = error;
        Args = args;
    }

    internal SFError Error { get; set; }

    /// <summary>Values formatted into the message of <see cref="Error"/>.</summary>
    internal object[] Args { get; }

    public override Exception ToPublicException()
    {
        var snowflakeDbException = InnerException is null
            ? new SnowflakeDbException(Error, Args)
            : new SnowflakeDbException(InnerException, Error, Args);

        if (QueryId is not null)
            snowflakeDbException.QueryId = QueryId;

        return new OperationCanceledException(snowflakeDbException.Message, snowflakeDbException);
    }
}
