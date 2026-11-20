using System.Runtime.InteropServices;
using Apache.Arrow.C;
using Snowflake.Data.Interop;

namespace Snowflake.Data.Tests.Core;

/// <summary>
/// <see cref="ArrowArrayStreamAbi"/> and <see cref="ArrowArrayAbi"/> lay out their fields as the Arrow C stream and
/// C data interfaces declare them, and as Apache.Arrow's own <see cref="CArrowArrayStream"/> and
/// <see cref="CArrowArray"/> do.
/// </summary>
[Trait("Category", "Unit")]
public sealed class ArrowAbiLayoutTest
{
    private const int Int64Size = sizeof(long);

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(nameof(ArrowArrayStreamAbi.GetSchema), "get_schema")]
    [InlineData(nameof(ArrowArrayStreamAbi.GetNext), "get_next")]
    [InlineData(nameof(ArrowArrayStreamAbi.GetLastError), "get_last_error")]
    [InlineData(nameof(ArrowArrayStreamAbi.Release), "release")]
    [InlineData(nameof(ArrowArrayStreamAbi.PrivateData), "private_data")]
    public void TestStreamFieldSitsAtApacheArrowOffset(string field, string apacheArrowField)
    {
        Assert.Equal(Marshal.OffsetOf<CArrowArrayStream>(apacheArrowField), Marshal.OffsetOf<ArrowArrayStreamAbi>(field));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(nameof(ArrowArrayStreamAbi.GetSchema), 0)]
    [InlineData(nameof(ArrowArrayStreamAbi.GetNext), 1)]
    [InlineData(nameof(ArrowArrayStreamAbi.GetLastError), 2)]
    [InlineData(nameof(ArrowArrayStreamAbi.Release), 3)]
    [InlineData(nameof(ArrowArrayStreamAbi.PrivateData), 4)]
    public void TestStreamFieldIsPointerAtCDeclarationIndex(string field, int pointerIndex)
    {
        Assert.Equal(pointerIndex * nint.Size, Marshal.OffsetOf<ArrowArrayStreamAbi>(field).ToInt32());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestStreamIsFivePointersLikeApacheArrowStream()
    {
        Assert.Equal(5 * nint.Size, Marshal.SizeOf<ArrowArrayStreamAbi>());
        Assert.Equal(Marshal.SizeOf<CArrowArrayStream>(), Marshal.SizeOf<ArrowArrayStreamAbi>());
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(nameof(ArrowArrayAbi.Length), "length")]
    [InlineData(nameof(ArrowArrayAbi.NullCount), "null_count")]
    [InlineData(nameof(ArrowArrayAbi.Offset), "offset")]
    [InlineData(nameof(ArrowArrayAbi.BufferCount), "n_buffers")]
    [InlineData(nameof(ArrowArrayAbi.ChildCount), "n_children")]
    [InlineData(nameof(ArrowArrayAbi.Buffers), "buffers")]
    [InlineData(nameof(ArrowArrayAbi.Children), "children")]
    [InlineData(nameof(ArrowArrayAbi.Dictionary), "dictionary")]
    [InlineData(nameof(ArrowArrayAbi.Release), "release")]
    [InlineData(nameof(ArrowArrayAbi.PrivateData), "private_data")]
    public void TestArrayFieldSitsAtApacheArrowOffset(string field, string apacheArrowField)
    {
        Assert.Equal(Marshal.OffsetOf<CArrowArray>(apacheArrowField), Marshal.OffsetOf<ArrowArrayAbi>(field));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(nameof(ArrowArrayAbi.Length), 0, 0)]
    [InlineData(nameof(ArrowArrayAbi.NullCount), 1, 0)]
    [InlineData(nameof(ArrowArrayAbi.Offset), 2, 0)]
    [InlineData(nameof(ArrowArrayAbi.BufferCount), 3, 0)]
    [InlineData(nameof(ArrowArrayAbi.ChildCount), 4, 0)]
    [InlineData(nameof(ArrowArrayAbi.Buffers), 5, 0)]
    [InlineData(nameof(ArrowArrayAbi.Children), 5, 1)]
    [InlineData(nameof(ArrowArrayAbi.Dictionary), 5, 2)]
    [InlineData(nameof(ArrowArrayAbi.Release), 5, 3)]
    [InlineData(nameof(ArrowArrayAbi.PrivateData), 5, 4)]
    public void TestArrayFieldFollowsInt64sAndPointersDeclaredBeforeIt(string field, int int64sBefore, int pointersBefore)
    {
        var expected = (int64sBefore * Int64Size) + (pointersBefore * nint.Size);

        Assert.Equal(expected, Marshal.OffsetOf<ArrowArrayAbi>(field).ToInt32());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestArrayIsAsLargeAsApacheArrowArray()
    {
        Assert.Equal(Marshal.SizeOf<CArrowArray>(), Marshal.SizeOf<ArrowArrayAbi>());
    }
}
