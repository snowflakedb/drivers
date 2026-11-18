using System.Runtime.InteropServices;

namespace Snowflake.Data.Interop;

/// <summary>
/// The layout of the Arrow C stream interface's <c>struct ArrowArrayStream</c>.
/// </summary>
/// <remarks>
/// In the Apache.Arrow version the driver references, <c>CArrowArrayStream</c> keeps its callbacks internal, so the
/// driver reads them through this layout. Pointers are held as <see cref="nint"/>; a stream whose <see cref="Release"/> is zero is released.
/// </remarks>
[StructLayout(LayoutKind.Sequential)]
internal struct ArrowArrayStreamAbi
{
    /// <summary><c>int (*get_schema)(struct ArrowArrayStream*, struct ArrowSchema* out)</c>.</summary>
    public nint GetSchema;

    /// <summary><c>int (*get_next)(struct ArrowArrayStream*, struct ArrowArray* out)</c>.</summary>
    public nint GetNext;

    /// <summary><c>const char* (*get_last_error)(struct ArrowArrayStream*)</c>.</summary>
    public nint GetLastError;

    /// <summary><c>void (*release)(struct ArrowArrayStream*)</c>.</summary>
    public nint Release;

    /// <summary>Producer-owned data.</summary>
    public nint PrivateData;
}

/// <summary>
/// The layout of the Arrow C data interface's <c>struct ArrowArray</c>, read to tell a released array apart.
/// </summary>
/// <remarks>In the Apache.Arrow version the driver references, <c>CArrowArray</c> keeps its <c>release</c> callback internal.</remarks>
[StructLayout(LayoutKind.Sequential)]
internal struct ArrowArrayAbi
{
    /// <summary><c>int64_t length</c>.</summary>
    public long Length;

    /// <summary><c>int64_t null_count</c>.</summary>
    public long NullCount;

    /// <summary><c>int64_t offset</c>.</summary>
    public long Offset;

    /// <summary><c>int64_t n_buffers</c>.</summary>
    public long BufferCount;

    /// <summary><c>int64_t n_children</c>.</summary>
    public long ChildCount;

    /// <summary><c>const void** buffers</c>.</summary>
    public nint Buffers;

    /// <summary><c>struct ArrowArray** children</c>.</summary>
    public nint Children;

    /// <summary><c>struct ArrowArray* dictionary</c>.</summary>
    public nint Dictionary;

    /// <summary><c>void (*release)(struct ArrowArray*)</c>; zero when the array is released.</summary>
    public nint Release;

    /// <summary>Producer-owned data.</summary>
    public nint PrivateData;
}
