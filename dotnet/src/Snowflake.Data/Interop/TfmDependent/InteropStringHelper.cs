using System.Runtime.InteropServices;

namespace Snowflake.Data.Interop.TfmDependent;

#if !NETFRAMEWORK
internal sealed unsafe class InteropStringHelper : IInteropStringHelper
{
    /// <summary>The most bytes read from a native string; a longer string is cut off at this length.</summary>
    internal const int MaxBytes = 1 << 20;

    internal static IInteropStringHelper Instance { get; } = new InteropStringHelper();

    public string PtrToStringUtf8(byte* ptr)
    {
        if (ptr == null)
            return string.Empty;

        var len = 0;
        while (len < MaxBytes && ptr[len] != 0)
            len++;

        return len == 0
            ? string.Empty
            : Marshal.PtrToStringUTF8((nint)ptr, len);
    }
}

#endif
