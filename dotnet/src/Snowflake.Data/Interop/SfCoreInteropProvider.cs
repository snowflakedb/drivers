#if NETFRAMEWORK
using Snowflake.Data.Interop.TfmDependent.Framework;
#else
using Snowflake.Data.Interop.TfmDependent;
#endif

namespace Snowflake.Data.Interop;

internal static class SfCoreInteropProvider
{
    internal static readonly ISfCoreInterop SfCore = SfCoreNativeMethods.Instance;

    internal static readonly IInteropStringHelper StringHelper = InteropStringHelper.Instance;
}
