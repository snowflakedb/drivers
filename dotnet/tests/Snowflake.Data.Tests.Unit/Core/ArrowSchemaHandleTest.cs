using System.Runtime.InteropServices;
using Apache.Arrow;
using Apache.Arrow.C;
using Apache.Arrow.Types;
using Snowflake.Data.Interop;
using Snowflake.Data.Interop.Handles;

namespace Snowflake.Data.Tests.Core;

[Trait("Category", "Unit")]
public sealed class ArrowSchemaHandleTest
{
    private readonly RecordBatch _batch = UnitBatches.Batch(UnitBatches.Int64(1), UnitBatches.Strings("a"));

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestUnfilledHandleOwnsStruct()
    {
        using var handle = new ArrowSchemaHandle();

        Assert.False(handle.IsInvalid);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestSchemaProducerWritesImportsBackUnchanged()
    {
        using var stream = NativeArrowStreamTest.Filled(new FakeArrowArrayStream(_batch.Schema));
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var lease));
        using (lease)
        using (var handle = new ArrowSchemaHandle())
        {
            Assert.Equal(0, lease!.GetSchema(handle));

            var schema = handle.Import();

            Assert.Equal(["C1", "C2"], schema.FieldsList.Select(field => field.Name));
            Assert.IsType<Int64Type>(schema.FieldsList[0].DataType);
            Assert.IsType<StringType>(schema.FieldsList[1].DataType);
            Assert.All(schema.FieldsList, field => Assert.True(field.IsNullable));
        }
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestImportingReleasesProducerCopySoSecondImportIsRejected()
    {
        using var stream = NativeArrowStreamTest.Filled(new FakeArrowArrayStream(_batch.Schema));
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var lease));
        using (lease)
        using (var handle = new ArrowSchemaHandle())
        {
            Assert.Equal(0, lease!.GetSchema(handle));
            handle.Import();

            Assert.Throws<ArgumentException>(handle.Import);
        }
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestDisposingFilledHandleNeverImportedClosesIt()
    {
        using var stream = NativeArrowStreamTest.Filled(new FakeArrowArrayStream(_batch.Schema));
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var lease));
        var handle = new ArrowSchemaHandle();
        using (lease)
            Assert.Equal(0, lease!.GetSchema(handle));

        handle.Dispose();

        Assert.True(handle.IsClosed);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestDisposingHandleTwiceClosesItOnce()
    {
        var handle = new ArrowSchemaHandle();

        handle.Dispose();
        handle.Dispose();

        Assert.True(handle.IsClosed);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestDisposingHandleCallsProducerReleaseOnce()
    {
        var release = CountingArrowCallbacks.ForSchema();
        var handle = new ArrowSchemaHandle();
        Marshal.WriteIntPtr(
            handle.DangerousGetHandle(),
            Marshal.OffsetOf<CArrowSchema>("release").ToInt32(),
            release.Pointer);

        handle.Dispose();
        handle.Dispose();

        Assert.Equal(1, release.Calls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestHandleOverZeroAddressIsInvalidAndRejectsImport()
    {
        using var handle = new ArrowSchemaHandle(0);

        Assert.True(handle.IsInvalid);
        Assert.Throws<InvalidOperationException>(handle.Import);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestClosedHandleRejectsImport()
    {
        var handle = new ArrowSchemaHandle();
        handle.Dispose();

        Assert.Throws<ObjectDisposedException>(handle.Import);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ConcurrentDisposeInvokesReleaseCallbackOnce()
    {
        var release = CountingArrowCallbacks.ForSchema();
        var handle = new ArrowSchemaHandle();
        Marshal.WriteIntPtr(
            handle.DangerousGetHandle(),
            Marshal.OffsetOf<CArrowSchema>("release").ToInt32(),
            release.Pointer);

        Parallel.For(0, 10, _ =>
        {
            handle.Dispose();
        });

        Assert.Equal(1, release.Calls);
    }
}
