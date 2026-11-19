using System.Runtime.InteropServices;

namespace Snowflake.Data.Compatibility;

internal static class MemoryMarshallHelper
{
    internal static void Write(Span<byte> bytes, nint address)
    {
#if NETFRAMEWORK
        var uAddress = (uint)address;
        MemoryMarshal.Write(bytes, ref uAddress);
#else
        MemoryMarshal.Write(bytes, (uint)address);
#endif
    }
}
