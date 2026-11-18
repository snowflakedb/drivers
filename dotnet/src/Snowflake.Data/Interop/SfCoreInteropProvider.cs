using Snowflake.Data.Interop.TfmDependent;

namespace Snowflake.Data.Interop;

internal static class SfCoreInteropProvider
{
    internal static readonly ISfCoreInterop SfCore = SfCoreNativeMethods.Instance;

    internal static readonly IInteropStringHelper StringHelper = InteropStringHelper.Instance;
}
