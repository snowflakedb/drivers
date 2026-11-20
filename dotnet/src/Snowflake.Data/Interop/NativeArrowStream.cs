using System.Runtime.InteropServices;
using Apache.Arrow.C;
using Microsoft.Extensions.Logging;
using Snowflake.Data.Compatibility;
using Snowflake.Data.Core;
using Snowflake.Data.Interop.Handles;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Interop;

/// <summary>
/// An Arrow C stream (<c>struct ArrowArrayStream</c>) owned by the driver and used by one caller at a time: either
/// a struct the driver allocates and sf_core fills, or a live stream sf_core allocates and hands over.
/// </summary>
/// <remarks>
/// <para>
/// sf_core fills a driver-allocated struct during ResultSetGetStream, at the address <see cref="ToProto"/> encodes;
/// the stream is not shared before that call completes. Every later use goes through a <see cref="Lease"/> from
/// <see cref="TryEnter"/>. Only one lease exists at a time, so <c>get_next</c> never runs twice at once, whether
/// the driver calls it directly or sf_core calls it for ArrowStreamGetNext.
/// </para>
/// <para>
/// A lease also holds a reference on the handle. Disposing the stream while a lease is open marks it closed; the
/// stream's <c>release</c> callback, and freeing a struct the driver allocated, run when the lease ends, so they never
/// run under a <c>get_next</c> in progress. The finalizer releases a stream that is never disposed.
/// </para>
/// </remarks>
internal sealed unsafe class NativeArrowStream : SafeHandle
{
    private readonly ILogger _logger;
    private readonly bool _selfAllocated;
    private int _busy;
    private int _disposed;

    internal NativeArrowStream() : this(LoggerAndTelemetryStubs.GetLogger<NativeArrowStream>())
    {
    }

    /// <summary>
    /// Takes ownership of the live stream at <paramref name="streamPtr"/>, a non-zero address of a struct sf_core
    /// allocated and handed over to the driver. Disposing the handle calls the stream's <c>release</c> and leaves the
    /// struct allocated.
    /// </summary>
    /// <exception cref="ArgumentException">The stream is already released.</exception>
    internal NativeArrowStream(nint streamPtr) : this(streamPtr, LoggerAndTelemetryStubs.GetLogger<NativeArrowStream>(), false)
    {
    }

    /// <summary>Allocates an empty, zeroed stream struct for sf_core to fill.</summary>
    internal NativeArrowStream(ILogger logger) : this((nint)CArrowArrayStream.Create(), logger, true)
    {
    }

    /// <summary>Takes ownership of the stream struct at <paramref name="streamPtr"/>.</summary>
    /// <param name="streamPtr">The struct's address.</param>
    /// <param name="logger">Receives the critical message when the stream is released while in use.</param>
    /// <param name="selfAllocated">
    /// True when <see cref="CArrowArrayStream.Create"/> allocated the struct: it may be unfilled or already released,
    /// and a zero address makes a handle that is invalid from the start. False when sf_core allocated it: the address
    /// is non-zero and the stream is live.
    /// </param>
    /// <exception cref="ArgumentException"><paramref name="selfAllocated"/> is false and the stream is already released.</exception>
    internal NativeArrowStream(nint streamPtr, ILogger logger, bool selfAllocated) : base(nint.Zero, ownsHandle: true)
    {
        _selfAllocated = selfAllocated;
        if (!_selfAllocated)
        {
            var cPtr = (ArrowArrayStreamAbi*)streamPtr;
            if (cPtr->Release == nint.Zero)
                throw new ArgumentException("Tried to import an array stream that has already been released.");
        }
        _logger = logger;
        SetHandle(streamPtr);
    }

    public override bool IsInvalid => handle == nint.Zero;

    /// <summary>
    /// Whether the stream has been disposed. Unlike <see cref="SafeHandle.IsClosed"/>, this is true as soon as
    /// <see cref="IDisposable.Dispose"/> runs, also while a lease defers the release.
    /// </summary>
    internal bool IsDisposed => Volatile.Read(ref _disposed) != 0;

    /// <summary>The struct's address in the form sf_core reads it: 8 bytes, little-endian.</summary>
    /// <exception cref="ObjectDisposedException">The stream's memory is already freed.</exception>
    internal ArrowArrayStreamPtr ToProto()
    {
        if (IsClosed)
            throw new ObjectDisposedException(nameof(NativeArrowStream));

        return new() { Value = ArrowPtrCodec.Encode64(handle) };
    }

    /// <summary>Starts a lease of the stream, unless another lease is in progress or the stream is released.</summary>
    /// <param name="lease">The started lease when the result is <see cref="NativeArrowStreamEnterResult.Entered"/>; otherwise null.</param>
    /// <remarks>
    /// The reference is added before the lease is taken and dropped after the lease is given back, so a lease is only
    /// ever held together with a reference.
    /// </remarks>
    internal NativeArrowStreamEnterResult TryEnter(out Lease? lease)
    {
        lease = null;
        if (IsDisposed || IsInvalid)
            return NativeArrowStreamEnterResult.Released;

        var referenced = false;
        try
        {
            DangerousAddRef(ref referenced);
        }
        catch (ObjectDisposedException)
        {
            return NativeArrowStreamEnterResult.Released;
        }

        var result = IsDisposed ? NativeArrowStreamEnterResult.Released : TryTakeLease();
        if (result == NativeArrowStreamEnterResult.Entered)
        {
            lease = new Lease(this);
            return result;
        }

        DangerousRelease();
        return result;
    }

    protected override void Dispose(bool disposing)
    {
        Volatile.Write(ref _disposed, 1);
        base.Dispose(disposing);
    }

    /// <remarks>
    /// Runs once the handle is closed and its last reference is gone, so it takes the use without adding a
    /// reference. A use is only held together with a reference, so no lease and no <c>get_next</c> can be in
    /// progress here. If the stream is in use anyway, the struct is left allocated rather than freed under that use,
    /// and the producer's <c>release</c> does not run: that memory leaks for the life of the process, and the driver
    /// cannot recover it because nothing retries the release.
    /// A live stream has its <c>release</c> called. The driver frees the struct only when it allocated it; a struct
    /// sf_core allocated stays allocated, released, after the handle closes.
    /// </remarks>
    protected override bool ReleaseHandle()
    {
        var lease = TryTakeLease();
        switch (lease)
        {
            case NativeArrowStreamEnterResult.Entered:
                ArrowStreamCalls.Release((ArrowArrayStreamAbi*)handle);
                if (!_selfAllocated)
                    return true;
                break;
            case NativeArrowStreamEnterResult.Busy:
                _logger.LogCritical("The Arrow stream was released while a use of it was in progress. The stream's memory and its producer's resources leak and cannot be reclaimed");
                return false;
            case NativeArrowStreamEnterResult.Released:
                if (!_selfAllocated)
                    return false;
                break;
            default:
                throw new ArgumentOutOfRangeException(nameof(lease), lease, "The Arrow stream's use state is not one of the known states.");
        }

        CArrowArrayStream.Free((CArrowArrayStream*)handle);

        return true;
    }

    /// <summary>
    /// Takes the use of the stream without adding a reference: <see cref="NativeArrowStreamEnterResult.Busy"/> while
    /// another use holds it, <see cref="NativeArrowStreamEnterResult.Released"/> without taking it once the producer
    /// has released the stream.
    /// </summary>
    private NativeArrowStreamEnterResult TryTakeLease()
    {
        if (Interlocked.CompareExchange(ref _busy, 1, 0) != 0)
            return NativeArrowStreamEnterResult.Busy;

        if (((ArrowArrayStreamAbi*)handle)->Release != 0)
            return NativeArrowStreamEnterResult.Entered;

        Volatile.Write(ref _busy, 0);
        return NativeArrowStreamEnterResult.Released;
    }

    private void Exit()
    {
        Volatile.Write(ref _busy, 0);
        DangerousRelease();
    }

    /// <summary>One use of a <see cref="NativeArrowStream"/>; disposing it ends the use.</summary>
    internal sealed class Lease : IDisposable
    {
        private NativeArrowStream? _stream;

        internal Lease(NativeArrowStream stream)
        {
            _stream = stream;
        }

        private NativeArrowStream Stream => _stream ?? throw new ObjectDisposedException(nameof(Lease));

        public void Dispose() => Interlocked.Exchange(ref _stream, null)?.Exit();

        /// <summary>The stream's address in the form sf_core reads it: 8 bytes, little-endian.</summary>
        internal ArrowArrayStreamPtr ToProto() => Stream.ToProto();

        /// <summary>Calls the stream's <c>get_schema</c> with <paramref name="handle"/>; returns its <c>errno</c>-compatible status.</summary>
        /// <exception cref="InvalidOperationException">The stream has no <c>get_schema</c> callback.</exception>
        internal int GetSchema(ArrowSchemaHandle handle)
        {
            var abi = (ArrowArrayStreamAbi*)Stream.handle;
            if (abi->GetSchema == 0)
                throw new InvalidOperationException("The Arrow stream has no get_schema callback.");
            try
            {
                return ArrowStreamCalls.GetSchema(abi, handle);
            }
            finally
            {
                GC.KeepAlive(handle);
            }
        }

        /// <summary>
        /// Calls the stream's <c>get_next</c> with <paramref name="handle"/> on the calling thread, which may block until
        /// the next chunk has downloaded; returns its <c>errno</c>-compatible status.
        /// </summary>
        /// <exception cref="InvalidOperationException">The stream has no <c>get_next</c> callback.</exception>
        internal int GetNext(ArrowArrayHandle handle)
        {
            var abi = (ArrowArrayStreamAbi*)Stream.handle;
            if (abi->GetNext == 0)
                throw new InvalidOperationException("The Arrow stream has no get_next callback.");
            try
            {
                return ArrowStreamCalls.GetNext(abi, handle);
            }
            finally
            {
                GC.KeepAlive(handle);
            }
        }

        /// <summary>The stream's description of its last failure; valid only right after a call that returned a non-zero status.</summary>
        internal string GetLastError() => ArrowStreamCalls.GetLastError((ArrowArrayStreamAbi*)Stream.handle) ?? string.Empty;
    }
}
