using System.Buffers.Binary;
using System.Runtime.InteropServices;
using Apache.Arrow;
using Snowflake.Data.Interop;
using Snowflake.Data.Interop.Handles;
using Snowflake.Data.Tests.Utilities;

namespace Snowflake.Data.Tests.Core;

[Trait("Category", "Unit")]
public sealed class ArrowArrayHandleTest
{
    private readonly RecordBatch _batch = UnitBatches.Batch(UnitBatches.Int64(7, null, 9));

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestUnfilledHandleOwnsStructThatHoldsNoBatch()
    {
        using var handle = new ArrowArrayHandle();

        Assert.False(handle.IsInvalid);
        Assert.True(handle.IsReleased);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestProtoCarriesHandleOwnAddress()
    {
        using var handle = new ArrowArrayHandle();

        var encoded = handle.ToProto().Value;

        Assert.Equal(sizeof(ulong), encoded.Length);
        Assert.Equal(unchecked((ulong)(nuint)(nint)handle.DangerousGetHandle()), (nuint)(nint)BinaryPrimitives.ReadUInt64LittleEndian(encoded.Span));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestHandleOverZeroAddressIsInvalidAndClosesWithoutFreeing()
    {
        var handle = new ArrowArrayHandle(0);

        Assert.True(handle.IsInvalid);
        handle.Dispose();
        Assert.True(handle.IsClosed);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestFilledHandleHoldsBatchUntilImportMovesItOut()
    {
        using var stream = NativeArrowStreamTest.Filled(new FakeArrowArrayStream(_batch.Schema, _batch));
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var lease));
        using (lease)
        using (var handle = new ArrowArrayHandle())
        {
            Assert.Equal(0, lease!.GetNext(handle));
            Assert.False(handle.IsReleased);

            using var imported = handle.Import(_batch.Schema);

            Assert.True(handle.IsReleased);
            Assert.Equal(3, imported.Length);
            var values = Assert.IsType<Int64Array>(imported.Column(0));
            Assert.Equal([7L, null, 9L], [values.GetValue(0), values.GetValue(1), values.GetValue(2)]);
        }
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestDisposingFilledHandleNeverImportedClosesIt()
    {
        using var stream = NativeArrowStreamTest.Filled(new FakeArrowArrayStream(_batch.Schema, _batch));
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var lease));
        var handle = new ArrowArrayHandle();
        using (lease)
            Assert.Equal(0, lease!.GetNext(handle));

        handle.Dispose();

        Assert.True(handle.IsClosed);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestDisposingHandleTwiceClosesItOnce()
    {
        var handle = new ArrowArrayHandle();

        handle.Dispose();
        handle.Dispose();

        Assert.True(handle.IsClosed);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestDisposingHandleCallsProducerReleaseOnce()
    {
        var release = CountingArrowCallbacks.ForArray();
        var handle = new ArrowArrayHandle();
        Marshal.WriteIntPtr(
            handle.DangerousGetHandle(),
            Marshal.OffsetOf<ArrowArrayAbi>(nameof(ArrowArrayAbi.Release)).ToInt32(),
            release.Pointer);

        handle.Dispose();
        handle.Dispose();

        Assert.Equal(1, release.Calls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestClosedHandleIsReleasedAndRejectsProtoAndImport()
    {
        var handle = new ArrowArrayHandle();
        handle.Dispose();

        Assert.True(handle.IsReleased);
        Assert.Throws<ObjectDisposedException>(handle.ToProto);
        Assert.Throws<ObjectDisposedException>(() => handle.Import(_batch.Schema));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestHandleOverZeroAddressIsReleasedAndRejectsProtoAndImport()
    {
        using var handle = new ArrowArrayHandle(0);

        Assert.True(handle.IsReleased);
        Assert.Throws<InvalidOperationException>(handle.ToProto);
        Assert.Throws<InvalidOperationException>(() => handle.Import(_batch.Schema));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DisposingHandleThatHoldsBatchCallsTheProducersReleaseOnce()
    {
        var release = CountingArrowCallbacks.ForArray();
        var handle = new ArrowArrayHandle();
        Marshal.WriteIntPtr(
            handle.DangerousGetHandle(),
            Marshal.OffsetOf<ArrowArrayAbi>(nameof(ArrowArrayAbi.Release)).ToInt32(),
            release.Pointer);

        Parallel.For(0, 10, _ =>
        {
            handle.Dispose();
        });

        Assert.Equal(1, release.Calls);
    }
}
