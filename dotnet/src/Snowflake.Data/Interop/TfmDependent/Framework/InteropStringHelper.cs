using System.Text;

namespace Snowflake.Data.Interop.TfmDependent.Framework;

#if NETFRAMEWORK

internal sealed unsafe class InteropStringHelper : IInteropStringHelper
{
    internal static IInteropStringHelper Instance { get; } = new InteropStringHelper();

    public string PtrToStringUtf8(byte* ptr)
    {
        if (ptr == null)
            return string.Empty;

        var len = 0;
        while (ptr[len] != 0)
            len++;

        return len == 0
            ? string.Empty
            : Encoding.UTF8.GetString(ptr, len);
    }
}

#endif
