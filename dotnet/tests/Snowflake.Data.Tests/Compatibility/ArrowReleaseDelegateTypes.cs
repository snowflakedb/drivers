using System.Reflection;
using System.Runtime.InteropServices;
using Apache.Arrow.C;

namespace Snowflake.Data.Tests.Compatibility;

/// <summary>
/// The delegate types a <c>release</c> callback for an Arrow C Data Interface struct is created with, so that
/// Apache.Arrow's <c>Free</c> can call it on every target framework.
/// </summary>
/// <remarks>
/// On .NET Framework, <c>Free</c> calls <c>release</c> through
/// <see cref="Marshal.GetDelegateForFunctionPointer{TDelegate}(IntPtr)"/> with a delegate type internal to Apache.Arrow.
/// For a pointer from <see cref="Marshal.GetFunctionPointerForDelegate{TDelegate}(TDelegate)"/>, that method returns
/// the original delegate cast to the requested type, so the callback there is an instance of the internal type. On
/// later frameworks, <c>Free</c> calls the function pointer directly and any Cdecl delegate type with the same
/// signature works.
/// </remarks>
internal static unsafe class ArrowReleaseDelegateTypes
{
#if !NETFRAMEWORK
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate void ReleaseSchemaCallback(CArrowSchema* schema);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate void ReleaseArrayCallback(CArrowArray* array);
#endif

    /// <summary>The delegate type for a <c>release</c> callback taking a <see cref="CArrowSchema"/> pointer.</summary>
    internal static Type ForSchema() =>
#if NETFRAMEWORK
        ArrowDelegate(typeof(CArrowSchemaExporter), "ReleaseArrowSchema");
#else
        typeof(ReleaseSchemaCallback);
#endif

    /// <summary>The delegate type for a <c>release</c> callback taking a <see cref="CArrowArray"/> pointer.</summary>
    internal static Type ForArray() =>
#if NETFRAMEWORK
        ArrowDelegate(typeof(CArrowArrayExporter), "ReleaseArrowArray");
#else
        typeof(ReleaseArrayCallback);
#endif

#if NETFRAMEWORK
    private static Type ArrowDelegate(Type declaringType, string name) =>
        declaringType.GetNestedType(name, BindingFlags.NonPublic)
        ?? throw new MissingMemberException(declaringType.FullName, name);
#endif
}
