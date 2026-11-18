using Apache.Arrow.C;
using Snowflake.Data.Interop.Handles;
using Snowflake.Data.Interop.TfmDependent;

namespace Snowflake.Data.Interop;

/// <summary>
/// Calls the callbacks of an Arrow C stream (<c>struct ArrowArrayStream</c>) through unmanaged function pointers.
/// </summary>
/// <remarks>
/// The callbacks are C functions. The caller guarantees the stream is live, not released, and not used by
/// another call meanwhile; <see cref="NativeArrowStream"/> is the only caller in the driver.
/// </remarks>
internal static unsafe class ArrowStreamCalls
{
    /// <summary>Calls <c>get_schema</c>; returns its <c>errno</c>-compatible status.</summary>
    internal static int GetSchema(ArrowArrayStreamAbi* stream, ArrowSchemaHandle handle) =>
        ((delegate* unmanaged[Cdecl]<ArrowArrayStreamAbi*, CArrowSchema*, int>)(void*)stream->GetSchema)(stream, handle.Address);

    /// <summary>Calls <c>get_next</c>; returns its <c>errno</c>-compatible status.</summary>
    internal static int GetNext(ArrowArrayStreamAbi* stream, ArrowArrayHandle handle) =>
        ((delegate* unmanaged[Cdecl]<ArrowArrayStreamAbi*, CArrowArray*, int>)(void*)stream->GetNext)(stream, handle.Address);

    /// <summary>Calls <c>get_last_error</c>; null when the stream has no such callback or gives no text.</summary>
    internal static string? GetLastError(ArrowArrayStreamAbi* stream)
    {
        if (stream->GetLastError == 0)
            return null;

        var text = ((delegate* unmanaged[Cdecl]<ArrowArrayStreamAbi*, byte*>)(void*)stream->GetLastError)(stream);
        return text == null ? null : InteropStringHelper.Instance.PtrToStringUtf8(text);
    }

    /// <summary>
    /// Calls <c>release</c>, which frees the producer's resources and marks the stream released; the struct itself
    /// stays allocated.
    /// </summary>
    internal static void Release(ArrowArrayStreamAbi* stream) =>
        ((delegate* unmanaged[Cdecl]<ArrowArrayStreamAbi*, void>)(void*)stream->Release)(stream);
}
