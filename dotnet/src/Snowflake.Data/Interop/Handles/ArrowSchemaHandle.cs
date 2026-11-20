using System.Runtime.InteropServices;
using Apache.Arrow;
using Apache.Arrow.C;
using Snowflake.Data.Compatibility;

namespace Snowflake.Data.Interop.Handles;

/// <summary>
/// An unmanaged <c>struct ArrowSchema</c> that an Arrow C stream's <c>get_schema</c> fills in place, following the
/// Arrow C Data Interface convention that the consumer allocates the struct and the producer initializes it.
/// </summary>
/// <remarks>
/// Releasing the handle, on <see cref="IDisposable.Dispose"/> or by the finalizer, calls the producer's
/// <c>release</c> for a schema that was not imported, then frees the struct.
/// </remarks>
internal sealed unsafe class ArrowSchemaHandle : SafeHandle
{
    /// <summary>Allocates an empty, zeroed struct.</summary>
    internal ArrowSchemaHandle() : this((nint)CArrowSchema.Create())
    {
    }

    /// <summary>
    /// Takes ownership of the struct at <paramref name="ptr"/>, which <see cref="CArrowSchema.Create"/> allocated.
    /// </summary>
    internal ArrowSchemaHandle(nint ptr) : base(nint.Zero, ownsHandle: true)
    {
        SetHandle(ptr);
    }

    public override bool IsInvalid => handle == nint.Zero;

    /// <summary>The handle's address, for a stream's <c>get_schema</c>.</summary>
    /// <exception cref="ObjectDisposedException">The handle is closed.</exception>
    /// <exception cref="InvalidOperationException">The handle is over the zero address.</exception>
    internal CArrowSchema* Address
    {
        get
        {
            if (IsClosed)
                throw new ObjectDisposedException(nameof(ArrowSchemaHandle));
            if (IsInvalid)
                throw new InvalidOperationException("The handle is over the zero address.");

            return (CArrowSchema*)handle;
        }
    }

    /// <summary>Imports the schema the producer wrote into the handle and releases the producer's copy.</summary>
    internal Schema Import() => CArrowSchemaImporter.ImportSchema(Address);

    protected override bool ReleaseHandle()
    {
        CArrowSchema.Free((CArrowSchema*)handle);
        return true;
    }
}
