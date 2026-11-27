using Snowflake.Data.Client;
using Snowflake.Data.Core.Extensions;

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

    /// <remarks>
    /// The result carries the token of the <see cref="OperationCanceledException"/> that caused the cancellation,
    /// and <see cref="CancellationToken.None"/> when the cause is another exception or there is none.
    /// </remarks>
    public override Exception ToPublicException()
    {
        var snowflakeDbException = new SnowflakeDbException(null, Error.GetErrorCode(), Error.FormatMessage(Args), QueryId, InnerException);
        var token = InnerException is OperationCanceledException cancelled ? cancelled.CancellationToken : CancellationToken.None;

        return new OperationCanceledException(snowflakeDbException.Message, snowflakeDbException, token);
    }
}
