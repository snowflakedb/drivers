using System.Reflection;
using System.Runtime.InteropServices;
using Apache.Arrow.C;
using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests.Core;

/// <summary>
/// A <c>release</c> callback for an Arrow C Data Interface struct that counts the calls Apache.Arrow's <c>Free</c> makes.
/// </summary>
/// <remarks>The callback's delegate type comes from <see cref="ArrowReleaseDelegateTypes"/>.</remarks>
internal sealed unsafe class CountingArrowCallbacks
{
    private int _calls;

    private CountingArrowCallbacks(Type delegateType, string method)
    {
        Callback = typeof(CountingArrowCallbacks)
            .GetMethod(method, BindingFlags.Instance | BindingFlags.NonPublic)!
            .CreateDelegate(delegateType, this);
        Pointer = Marshal.GetFunctionPointerForDelegate(Callback);
    }

    /// <summary>The function pointer to write into the struct's <c>release</c> field; valid while this instance is reachable.</summary>
    internal nint Pointer { get; }

    /// <summary>The number of times the callback has run.</summary>
    internal int Calls => Volatile.Read(ref _calls);

    private Delegate Callback { get; }

    /// <summary>A callback for <see cref="CArrowSchema"/>.</summary>
    internal static CountingArrowCallbacks ForSchema() => new(ArrowReleaseDelegateTypes.ForSchema(), nameof(ReleaseSchema));

    /// <summary>A callback for <see cref="CArrowArray"/>.</summary>
    internal static CountingArrowCallbacks ForArray() => new(ArrowReleaseDelegateTypes.ForArray(), nameof(ReleaseArray));

    private void ReleaseSchema(CArrowSchema* _) => Interlocked.Increment(ref _calls);

    private void ReleaseArray(CArrowArray* _) => Interlocked.Increment(ref _calls);
}
