using System.Runtime.InteropServices;

namespace Snowflake.Data.Interop.TfmDependent;

#if !NETFRAMEWORK
internal sealed unsafe class InteropStringHelper : IInteropStringHelper
{
    internal static IInteropStringHelper Instance { get; } = new InteropStringHelper();

    public string PtrToStringUtf8(byte* ptr) => Marshal.PtrToStringUTF8((nint)ptr) ?? string.Empty;
}

#endif
