#if NET7_0_OR_GREATER
#else
using Snowflake.Data.Interop.Framework;
#endif

namespace Snowflake.Data.Interop;

internal static class NativeInteropProvider
{
    internal static readonly INativeInterop Interop = NativeInterop.Instance;
}
