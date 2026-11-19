using Google.Protobuf;
using Snowflake.Data.Interop;
using Snowflake.Data.Interop.Extensions;
using Snowflake.Data.Proto;
using Snowflake.Data.Tests.Utilities;

namespace Snowflake.Data.Tests.Core;

[Trait("Category", "Unit")]
[Collection(NativeMemorySensitiveCollection.Name)]
public sealed class PrepareResultExtensionsTest
{
    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestReleasingStreamCallsProducerReleaseOnce()
    {
        var producer = new FakeArrowArrayStream(UnitBatches.Batch(UnitBatches.Int64(1)).Schema);
        var address = ArrowStreamExporter.Export(producer);
        var stream = UnitArrowStreams.NativeWidth(address);

        var prepareResult = new PrepareResult { Stream = stream };

        prepareResult.Release();

        Assert.Equal(1, producer.DisposeCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestReleasingStreamWhoseSchemaReadFailsCallsProducerReleaseOnce()
    {
        var producer = new FakeArrowArrayStream(UnitBatches.Batch(UnitBatches.Int64(1)).Schema)
        {
            FailSchemaWith = new InvalidOperationException("Failed to read schema"),
        };
        var address = ArrowStreamExporter.Export(producer);
        var stream = UnitArrowStreams.NativeWidth(address);
        var prepareResult = new PrepareResult { Stream = stream };

        Assert.Null(Record.Exception(prepareResult.Release));
        Assert.Equal(1, producer.DisposeCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestReleasingStreamLeavesStructAllocatedInReleasedState()
    {
        var producer = new FakeArrowArrayStream(UnitBatches.Batch(UnitBatches.Int64(1)).Schema);
        var address = ArrowStreamExporter.Export(producer);
        try
        {
            var prepareResult = new PrepareResult { Stream = new ArrowArrayStreamPtr { Value = ArrowPtrCodec.Encode64(address) } };
            Assert.False(ArrowStreamExporter.IsReleased(address));

            prepareResult.Release();

            Assert.True(ArrowStreamExporter.IsReleased(address));
        }
        finally
        {
            ArrowStreamExporter.Free(address);
        }
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestReleasingStreamClearsStream()
    {
        var producer = new FakeArrowArrayStream(UnitBatches.Batch(UnitBatches.Int64(1)).Schema);
        var stream = new ArrowArrayStreamPtr { Value = ArrowPtrCodec.Encode64(ArrowStreamExporter.Export(producer)) };
        var prepareResult = new PrepareResult { Stream = stream };

        prepareResult.Release();

        Assert.Null(prepareResult.Stream);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestReleasingStreamTwiceIsNoOp()
    {
        var producer = new FakeArrowArrayStream(UnitBatches.Batch(UnitBatches.Int64(1)).Schema);
        var stream = new ArrowArrayStreamPtr { Value = ArrowPtrCodec.Encode64(ArrowStreamExporter.Export(producer)) };
        var prepareResult = new PrepareResult { Stream = stream };
        prepareResult.Release();

        Assert.Null(Record.Exception(prepareResult.Release));
        Assert.Equal(1, producer.DisposeCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestReleasingZeroAddressDoesNothing()
    {
        var stream = new ArrowArrayStreamPtr { Value = ByteString.CopyFrom(new byte[sizeof(long)]) };
        var prepareResult = new PrepareResult { Stream = stream };

        Assert.Null(Record.Exception(prepareResult.Release));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestReleasingPointerWithoutValueDoesNothing()
    {
        var stream = new ArrowArrayStreamPtr();
        var prepareResult = new PrepareResult { Stream = stream };

        Assert.Null(Record.Exception(prepareResult.Release));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestReleasingMissingPointerDoesNothing()
    {
        ArrowArrayStreamPtr? stream = null;
        var prepareResult = new PrepareResult { Stream = stream };

        Assert.Null(Record.Exception(prepareResult.Release));
    }
}
