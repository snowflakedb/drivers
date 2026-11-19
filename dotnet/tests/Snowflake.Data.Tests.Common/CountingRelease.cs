using System.Runtime.InteropServices;

namespace Snowflake.Data.Tests;

/// <summary>
/// sf_core callback written into an Arrow C struct, which counts its calls and marks the struct
/// released as the C data interface requires.
/// </summary>
public sealed class CountedCallback
{
    private readonly int _releaseOffset;
    private int _calls;

    public CountedCallback(nint structPtr, int releaseOffset)
    {
        _releaseOffset = releaseOffset;
        Marshal.WriteIntPtr(structPtr, _releaseOffset, Marshal.GetFunctionPointerForDelegate((Callback)CountAndRun));
    }

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate void Callback(nint structPtr);

    public int Calls => Volatile.Read(ref _calls);

    private void CountAndRun(nint structPtr)
    {
        Interlocked.Increment(ref _calls);
        Marshal.WriteIntPtr(structPtr, _releaseOffset, IntPtr.Zero);
    }
}
