using System.Runtime.InteropServices;
using Apache.Arrow;
using Apache.Arrow.C;
using Snowflake.Data.Compatibility;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Interop.Handles;

/// <summary>
/// An unmanaged <c>struct ArrowArray</c> that an Arrow C stream's <c>get_next</c> fills in place with one record batch.
/// </summary>
/// <remarks>
/// The struct has to stay allocated until the <c>get_next</c> it is passed to has returned, including when that
/// call runs inside sf_core. At the end of the stream <c>get_next</c> leaves it released (<see cref="IsReleased"/>).
/// Releasing the handle, on <see cref="IDisposable.Dispose"/> or by the finalizer, calls the producer's
/// <c>release</c> for a batch that was not imported, then frees the struct.
/// </remarks>
internal sealed unsafe class ArrowArrayHandle : SafeHandle
{
    /// <summary>Allocates an empty, zeroed struct.</summary>
    internal ArrowArrayHandle() : this((nint)CArrowArray.Create())
    {
    }

    /// <summary>
    /// Takes ownership of the struct at <paramref name="ptr"/>, which <see cref="CArrowArray.Create"/> allocated.
    /// </summary>
    internal ArrowArrayHandle(nint ptr) : base(nint.Zero, ownsHandle: true)
    {
        SetHandle(ptr);
    }

    public override bool IsInvalid => handle == nint.Zero;

    /// <summary>The handle's address, for a stream's <c>get_next</c>.</summary>
    /// <exception cref="ObjectDisposedException">The handle is closed.</exception>
    /// <exception cref="InvalidOperationException">The handle is over the zero address.</exception>
    internal CArrowArray* Address
    {
        get
        {
            ThrowIfUnusable();
            return (CArrowArray*)handle;
        }
    }

    /// <summary>
    /// Whether the handle holds no batch: it was never filled, the producer filled it with a released array, or the
    /// handle is closed or over the zero address.
    /// </summary>
    internal bool IsReleased => IsInvalid || IsClosed || ((ArrowArrayAbi*)handle)->Release == 0;

    /// <summary>The handle's address in the form sf_core reads it: 8 bytes, little-endian.</summary>
    /// <exception cref="ObjectDisposedException">The handle is closed.</exception>
    /// <exception cref="InvalidOperationException">The handle is over the zero address.</exception>
    internal ArrowArrayPtr ToProto()
    {
        ThrowIfUnusable();
        return new() { Value = ArrowPtrCodec.Encode64(handle) };
    }

    /// <summary>
    /// Moves the batch the producer wrote into the handle into a <see cref="RecordBatch"/>, which releases it
    /// once disposed.
    /// </summary>
    internal RecordBatch Import(Schema schema) => CArrowArrayImporter.ImportRecordBatch(Address, schema);

    protected override bool ReleaseHandle()
    {
        CArrowArray.Free((CArrowArray*)handle);
        return true;
    }

    private void ThrowIfUnusable()
    {
        if (IsClosed)
            throw new ObjectDisposedException(nameof(ArrowArrayHandle));
        if (IsInvalid)
            throw new InvalidOperationException("The handle is over the zero address.");
    }
}
