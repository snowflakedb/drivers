namespace Snowflake.Data.Proto.Errors;

/// <summary>
/// Recognizes the exceptions a call to sf_core raises when the call fails.
/// </summary>
/// <remarks>
/// The generated client raises <see cref="ServiceException"/> for an error sf_core reports and
/// <see cref="TransportException"/> when the call itself fails; an asynchronous call whose token is
/// cancelled ends with <see cref="OperationCanceledException"/>. Callers convert these into their own
/// error types. Any other exception did not come from sf_core and propagates unchanged.
/// This file is hand-written: <c>RegenerateProto</c> leaves the <c>Errors</c> folder in place.
/// </remarks>
public static class UniversalCoreExceptions
{
    /// <summary>True when <paramref name="exception"/> is a failure an sf_core call raises.</summary>
    public static bool IsUniversalCoreException(Exception exception) =>
        exception is ServiceException or TransportException or OperationCanceledException;
}
