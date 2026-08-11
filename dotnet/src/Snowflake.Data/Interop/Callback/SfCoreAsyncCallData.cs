using System.Runtime.InteropServices;

namespace Snowflake.Data.Interop.Callback;

internal sealed unsafe class SfCoreAsyncCallData : IDisposable
{
    public readonly TaskCompletionSource<SfCoreResponseData> TaskCompletionSource;
    private CancellationTokenRegistration? _cancelRegistration;
    private GCHandle _requestHandle;
    private GCHandle _selfHandle;
    private GCHandle _pinnedHandleHandle;
    private bool _isDisposed;

    private readonly LockObject _lock = new();

    public SfCoreAsyncCallData(TaskCompletionSource<SfCoreResponseData> taskCompletionSource, byte[] request)
    {
        _requestHandle = GCHandle.Alloc(request, GCHandleType.Pinned);
        TaskCompletionSource = taskCompletionSource;
    }

    /// <summary>
    /// Recovers the <see cref="SfCoreAsyncCallData"/> instance from the opaque pointer that was
    /// originally produced by <see cref="SelfPin"/> and passed to native code as <c>userData</c>.
    /// </summary>
    public static SfCoreAsyncCallData FromPtr(void* callDataPointer)
    {
        var gcHandleHandle = GCHandle.FromIntPtr((IntPtr)callDataPointer);
        var gcHandle = (GCHandle)gcHandleHandle.Target!;
        return (SfCoreAsyncCallData)gcHandle.Target!;
    }

    /// <summary>
    /// Pins this instance so it can be safely passed as a <c>void* userData</c> across the FFI
    /// boundary.  Uses double GCHandle indirection: a normal handle to <c>this</c> wrapped in a
    /// pinned handle to the struct, so the GC never moves the address we hand to native code.
    /// </summary>
    public void* SelfPin()
    {
        _selfHandle = GCHandle.Alloc(this);
        _pinnedHandleHandle = GCHandle.Alloc(_selfHandle, GCHandleType.Pinned);
        return (void*)GCHandle.ToIntPtr(_pinnedHandleHandle);
    }

    public void SetCancelRegistration(CancellationTokenRegistration? cancelRegistration)
    {
        lock (_lock)
        {
            if (_isDisposed)
                cancelRegistration?.Dispose();
            else
                _cancelRegistration = cancelRegistration;
        }
    }

    public byte* GetRequestPtr() => (byte*)_requestHandle.AddrOfPinnedObject();

    public void Dispose()
    {
        lock (_lock)
        {
            if (_isDisposed)
                return;

            _cancelRegistration?.Dispose();
            _requestHandle.Free();
            _pinnedHandleHandle.Free();
            _selfHandle.Free();
            _isDisposed = true;
        }
    }
}
