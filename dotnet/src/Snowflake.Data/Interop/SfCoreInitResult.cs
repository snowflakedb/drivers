using System.Runtime.InteropServices;

namespace Snowflake.Data.Interop;

/// <summary>
/// Return type of the native <c>sf_core_init</c> entry point (see <c>sf_core/src/logging/c_api.rs</c>).
/// </summary>
[StructLayout(LayoutKind.Sequential)]
internal struct SfCoreInitResult
{
    /// <summary>0 = success, non-zero = failure.</summary>
    public uint Status;

    /// <summary>1 if troubleshooting mode is active at init time, 0 otherwise.</summary>
    public uint TroubleshootingEnabled;
}
