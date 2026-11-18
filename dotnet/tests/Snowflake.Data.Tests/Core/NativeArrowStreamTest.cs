using System.Buffers.Binary;
using System.Reflection;
using System.Runtime.InteropServices;
using Apache.Arrow;
using Apache.Arrow.Types;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Logging.Abstractions;
using NSubstitute;
using Snowflake.Data.Compatibility;
using Snowflake.Data.Interop;
using Snowflake.Data.Interop.Extensions;
using Snowflake.Data.Interop.Handles;
using Snowflake.Data.Proto;
using Snowflake.Data.Tests.Utilities;

namespace Snowflake.Data.Tests.Core;

[Trait("Category", "Unit")]
public sealed class NativeArrowStreamTest
{
    internal const int ProducerFailureStatus = 131;

    private static readonly MethodInfo ReleaseHandleMethod =
        typeof(SafeHandle).GetMethod("ReleaseHandle", BindingFlags.Instance | BindingFlags.NonPublic)!;

    private readonly RecordBatch _batch = UnitBatches.Batch(UnitBatches.Int64(1, null), UnitBatches.Strings("a", "b"));
    private readonly ILogger _logger = Substitute.For<ILogger>();

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestStreamNobodyFilledCannotBeEntered()
    {
        using var stream = new NativeArrowStream(_logger);

        Assert.Equal(NativeArrowStreamEnterResult.Released, stream.TryEnter(out var lease));
        Assert.Null(lease);
        Assert.Equal(NativeArrowStreamEnterResult.Released, stream.TryEnter(out _));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestSchemaAndBatchesProducerWritesImportBackUnchangedThenEndIsReleasedArray()
    {
        using var stream = Filled(new FakeArrowArrayStream(_batch.Schema, _batch));
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var lease));
        using (lease)
        using (var schemaHandle = new ArrowSchemaHandle())
        using (var arrayHandle = new ArrowArrayHandle())
        using (var endHandle = new ArrowArrayHandle())
        {
            Assert.Equal(0, lease!.GetSchema(schemaHandle));
            var schema = schemaHandle.Import();
            Assert.Equal(0, lease.GetNext(arrayHandle));
            Assert.False(arrayHandle.IsReleased);
            using var imported = arrayHandle.Import(schema);
            Assert.Equal(0, lease.GetNext(endHandle));

            Assert.Equal(["C1", "C2"], schema.FieldsList.Select(field => field.Name));
            Assert.IsType<Int64Type>(schema.FieldsList[0].DataType);
            var ids = Assert.IsType<Int64Array>(imported.Column(0));
            Assert.Equal(1L, ids.GetValue(0));
            Assert.Null(ids.GetValue(1));
            Assert.Equal("b", Assert.IsType<StringArray>(imported.Column(1)).GetString(1));
            Assert.True(endHandle.IsReleased);
        }
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestFailedReadReportsProducerStatusAndText()
    {
        var producer = new FakeArrowArrayStream(_batch.Schema, _batch) { FailNextWith = new InvalidOperationException("Failed to read chunk data") };
        using var stream = Filled(producer);
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var lease));
        using var handle = new ArrowArrayHandle();

        using (lease)
        {
            Assert.Equal(ProducerFailureStatus, lease!.GetNext(handle));
            Assert.Contains("Failed to read chunk data", lease.GetLastError(), StringComparison.Ordinal);
        }

        Assert.True(handle.IsReleased);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestOnlyOneLeaseExistsPerStream()
    {
        using var stream = Filled(new FakeArrowArrayStream(_batch.Schema));

        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var first));
        Assert.Equal(NativeArrowStreamEnterResult.Busy, stream.TryEnter(out var second));
        Assert.Null(second);
        first!.Dispose();
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var third));
        third!.Dispose();
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestDisposingLeaseTwiceEndsOnlyItsOwnUse()
    {
        using var stream = Filled(new FakeArrowArrayStream(_batch.Schema));
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var first));
        first!.Dispose();
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var second));

        first.Dispose();

        Assert.Equal(NativeArrowStreamEnterResult.Busy, stream.TryEnter(out _));
        second!.Dispose();
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestDisposingStreamDuringUseDefersReleaseUntilUseEnds()
    {
        var producer = new FakeArrowArrayStream(_batch.Schema);
        var stream = Filled(producer, _logger);
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var lease));

        stream.Dispose();

        Assert.True(stream.IsDisposed);
        Assert.False(stream.IsClosed);
        Assert.Equal(0, producer.DisposeCalls);
        lease!.Dispose();
        Assert.Equal(1, producer.DisposeCalls);
        Assert.True(stream.IsClosed);
        Assert.Equal(NativeArrowStreamEnterResult.Released, stream.TryEnter(out _));
        Assert.Empty(_logger.ReceivedCalls());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestDisposingStreamReleasesProducerOnce()
    {
        var producer = new FakeArrowArrayStream(_batch.Schema);
        var stream = Filled(producer, _logger);

        stream.Dispose();
        stream.Dispose();

        Assert.Equal(1, producer.DisposeCalls);
        Assert.Equal(NativeArrowStreamEnterResult.Released, stream.TryEnter(out _));
        Assert.Empty(_logger.ReceivedCalls());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestDisposingStreamNobodyFilledFreesItWithoutLogging()
    {
        var stream = new NativeArrowStream(_logger);

        stream.Dispose();

        Assert.True(stream.IsClosed);
        Assert.Empty(_logger.ReceivedCalls());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task TestReleasingStreamWhileReadInProgressLogsCriticalAndLeavesProducerUnreleasedAsync()
    {
        using var gate = new SemaphoreSlim(0);
        var producer = new FakeArrowArrayStream(_batch.Schema, _batch) { Gate = gate };
        var stream = Filled(producer, _logger);
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var lease));
        using var handle = new ArrowArrayHandle();
        var read = Task.Run(() =>
        {
            using (lease)
                return lease!.GetNext(handle);
        });
        Assert.True(producer.ReadStarted.Wait(TimeSpan.FromSeconds(30)));

        var released = (bool)ReleaseHandleMethod.Invoke(stream, null)!;

        Assert.False(released);
        Assert.Equal(0, producer.DisposeCalls);
        var call = Assert.Single(_logger.ReceivedCalls());
        var arguments = call.GetArguments();
        Assert.Equal(LogLevel.Critical, Assert.IsType<LogLevel>(arguments[0]));
        Assert.Contains("leak", arguments[2]!.ToString(), StringComparison.Ordinal);

        gate.Release();
        Assert.Same(read, await Task.WhenAny(read, Task.Delay(TimeSpan.FromSeconds(30))).ConfigureAwait(false));
        Assert.Equal(0, await read.ConfigureAwait(false));
        stream.Dispose();
        Assert.True(stream.IsClosed);
        Assert.Equal(1, producer.DisposeCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestFailedEnterDoesNotKeepStreamOpenAfterDispose()
    {
        var producer = new FakeArrowArrayStream(_batch.Schema);
        var stream = Filled(producer);
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var lease));
        Assert.Equal(NativeArrowStreamEnterResult.Busy, stream.TryEnter(out _));
        lease!.Dispose();

        stream.Dispose();

        Assert.True(stream.IsClosed);
        Assert.Equal(1, producer.DisposeCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestConcurrentEnterAndDisposeReleasesProducerOnceWithoutErrors()
    {
        const int rounds = 200;
        const int enterers = 4;
        const int attemptsPerEnterer = 50;
        for (var round = 0; round < rounds; round++)
        {
            var producer = new FakeArrowArrayStream(_batch.Schema);
            var logger = Substitute.For<ILogger>();
            var stream = Filled(producer, logger);
            var failures = new List<Exception>();
            using var start = new Barrier(enterers + 1);
            var threads = Enumerable.Range(0, enterers).Select(_ => new Thread(() =>
            {
                start.SignalAndWait();
                try
                {
                    for (var attempt = 0; attempt < attemptsPerEnterer; attempt++)
                    {
                        var result = stream.TryEnter(out var lease);
                        lease?.Dispose();
                        if (result == NativeArrowStreamEnterResult.Released)
                            return;
                    }
                }
                catch (Exception exception)
                {
                    lock (failures)
                        failures.Add(exception);
                }
            })).ToList();
            threads.ForEach(thread => thread.Start());
            start.SignalAndWait();

            stream.Dispose();
            threads.ForEach(thread => Assert.True(thread.Join(TimeSpan.FromSeconds(30))));

            Assert.Empty(failures);
            Assert.True(stream.IsClosed);
            Assert.Equal(1, producer.DisposeCalls);
            Assert.Empty(logger.ReceivedCalls());
        }
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestFailedSchemaReadReportsProducerStatusAndText()
    {
        var producer = new FakeArrowArrayStream(_batch.Schema) { FailSchemaWith = new InvalidOperationException("Failed to read schema") };
        using var stream = Filled(producer);
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var lease));
        using var handle = new ArrowSchemaHandle();

        using (lease)
        {
            Assert.Equal(ProducerFailureStatus, lease!.GetSchema(handle));
            Assert.Contains("Failed to read schema", lease.GetLastError(), StringComparison.Ordinal);
        }
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestStreamWithoutGetSchemaCallbackRejectsGetSchema()
    {
        using var stream = Filled(new FakeArrowArrayStream(_batch.Schema));
        ClearCallback(stream, nameof(ArrowArrayStreamAbi.GetSchema));
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var lease));
        using var handle = new ArrowSchemaHandle();

        using (lease)
            Assert.Throws<InvalidOperationException>(() => lease!.GetSchema(handle));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestStreamWithoutGetNextCallbackRejectsGetNext()
    {
        using var stream = Filled(new FakeArrowArrayStream(_batch.Schema));
        ClearCallback(stream, nameof(ArrowArrayStreamAbi.GetNext));
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var lease));
        using var handle = new ArrowArrayHandle();

        using (lease)
            Assert.Throws<InvalidOperationException>(() => lease!.GetNext(handle));

        Assert.True(handle.IsReleased);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestProtoOfDisposedStreamIsRejected()
    {
        var stream = new NativeArrowStream(_logger);
        stream.Dispose();

        Assert.Throws<ObjectDisposedException>(stream.ToProto);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestStreamOverZeroAddressIsInvalidAndCannotBeEntered()
    {
        var stream = new NativeArrowStream(0, _logger, true);

        Assert.True(stream.IsInvalid);
        Assert.Equal(NativeArrowStreamEnterResult.Released, stream.TryEnter(out var lease));
        Assert.Null(lease);
        stream.Dispose();
        Assert.True(stream.IsClosed);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestProtoCarriesStreamOwnAddress()
    {
        using var stream = new NativeArrowStream(_logger);

        var encoded = stream.ToProto().Value;

        Assert.Equal(sizeof(ulong), encoded.Length);
        Assert.Equal(unchecked((ulong)(nuint)(nint)stream.DangerousGetHandle()), (nuint)(nint)BinaryPrimitives.ReadUInt64LittleEndian(encoded.Span));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestLeaseProtoIsStreamProto()
    {
        using var stream = Filled(new FakeArrowArrayStream(_batch.Schema));
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var lease));

        using (lease)
            Assert.Equal(stream.ToProto(), lease!.ToProto());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestStreamWithProducerAlreadyReleasedCannotBeEnteredAndIsNotReleasedAgain()
    {
        var producer = new FakeArrowArrayStream(_batch.Schema);
        var stream = Filled(producer);
        ArrowStreamExporter.ReleaseStreamAt(stream.ToProto().Value.Span);

        Assert.Equal(NativeArrowStreamEnterResult.Released, stream.TryEnter(out var lease));
        Assert.Null(lease);
        Assert.Equal(NativeArrowStreamEnterResult.Released, stream.TryEnter(out _));
        stream.Dispose();
        Assert.True(stream.IsClosed);
        Assert.Equal(1, producer.DisposeCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestSuccessfulReadLeavesNoErrorText()
    {
        using var stream = Filled(new FakeArrowArrayStream(_batch.Schema, _batch));
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var lease));
        using var handle = new ArrowArrayHandle();

        using (lease)
        {
            Assert.Equal(0, lease!.GetNext(handle));
            Assert.Equal(string.Empty, lease.GetLastError());
        }
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestStreamWithoutGetLastErrorCallbackReportsEmptyErrorText()
    {
        var producer = new FakeArrowArrayStream(_batch.Schema) { FailNextWith = new InvalidOperationException("Failed to read chunk data") };
        using var stream = Filled(producer);
        ClearCallback(stream, nameof(ArrowArrayStreamAbi.GetLastError));
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var lease));
        using var handle = new ArrowArrayHandle();

        using (lease)
        {
            Assert.Equal(ProducerFailureStatus, lease!.GetNext(handle));
            Assert.Equal(string.Empty, lease.GetLastError());
        }
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestEndedLeaseRejectsEveryCall()
    {
        using var stream = Filled(new FakeArrowArrayStream(_batch.Schema, _batch));
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var lease));
        lease!.Dispose();
        using var schemaHandle = new ArrowSchemaHandle();
        using var arrayHandle = new ArrowArrayHandle();

        Assert.Throws<ObjectDisposedException>(() => lease.GetSchema(schemaHandle));
        Assert.Throws<ObjectDisposedException>(() => lease.GetNext(arrayHandle));
        Assert.Throws<ObjectDisposedException>(lease.GetLastError);
        Assert.Throws<ObjectDisposedException>(lease.ToProto);
        Assert.True(arrayHandle.IsReleased);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestExactlyOneOfThreadsEnteringAtOnceGetsLease()
    {
        const int contenders = 8;
        using var stream = Filled(new FakeArrowArrayStream(_batch.Schema));
        using var start = new Barrier(contenders);
        var results = new NativeArrowStreamEnterResult[contenders];
        var leases = new NativeArrowStream.Lease?[contenders];
        var threads = Enumerable.Range(0, contenders).Select(i => new Thread(() =>
        {
            start.SignalAndWait();
            results[i] = stream.TryEnter(out leases[i]);
        })).ToList();

        threads.ForEach(thread => thread.Start());
        threads.ForEach(thread => Assert.True(thread.Join(TimeSpan.FromSeconds(30))));

        Assert.Equal(1, results.Count(result => result == NativeArrowStreamEnterResult.Entered));
        Assert.Equal(contenders - 1, results.Count(result => result == NativeArrowStreamEnterResult.Busy));
        Assert.Single(leases, lease => lease is not null);
        foreach (var lease in leases)
            lease?.Dispose();
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task TestReadInProgressOnAnotherThreadKeepsOtherUsesOutAndDefersReleaseUntilItReturnsAsync()
    {
        using var gate = new SemaphoreSlim(0);
        var producer = new FakeArrowArrayStream(_batch.Schema, _batch) { Gate = gate };
        var stream = Filled(producer, _logger);
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var lease));
        using var handle = new ArrowArrayHandle();
        var read = Task.Run(() =>
        {
            using (lease)
                return lease!.GetNext(handle);
        });
        Assert.True(producer.ReadStarted.Wait(TimeSpan.FromSeconds(30)));

        Assert.Equal(NativeArrowStreamEnterResult.Busy, stream.TryEnter(out _));
        stream.Dispose();
        Assert.Equal(NativeArrowStreamEnterResult.Released, stream.TryEnter(out _));
        Assert.False(stream.IsClosed);
        Assert.Equal(0, producer.DisposeCalls);

        gate.Release();
        Assert.Same(read, await Task.WhenAny(read, Task.Delay(TimeSpan.FromSeconds(30))).ConfigureAwait(false));
        Assert.Equal(0, await read.ConfigureAwait(false));
        Assert.False(handle.IsReleased);
        Assert.Equal(1, producer.DisposeCalls);
        Assert.True(stream.IsClosed);
        Assert.Empty(_logger.ReceivedCalls());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task TestReadThroughArrowStreamGetNextFillsRequestedArrayAsync()
    {
        var driver = Substitute.For<IDatabaseDriverService>();
        UnitStreamRpcs.ServeGetNext(driver);
        using var stream = Filled(new FakeArrowArrayStream(_batch.Schema, _batch));
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var lease));
        using var handle = new ArrowArrayHandle();

        using (lease)
        {
            var response = await driver.ArrowStreamGetNextBatchAsync(
                new ArrowStreamGetNextBatchRequest { Stream = lease!.ToProto(), OutArray = handle.ToProto() }, CancellationToken.None).ConfigureAwait(false);

            Assert.Equal(0, response.Status);
            Assert.False(response.HasErrorMessage);
        }

        using var imported = handle.Import(_batch.Schema);
        Assert.Equal(1L, Assert.IsType<Int64Array>(imported.Column(0)).GetValue(0));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task TestFailedReadThroughArrowStreamGetNextReportsStatusAndProducerTextAsync()
    {
        var driver = Substitute.For<IDatabaseDriverService>();
        UnitStreamRpcs.ServeGetNext(driver);
        var producer = new FakeArrowArrayStream(_batch.Schema, _batch) { FailNextWith = new InvalidOperationException("Failed to read chunk data") };
        using var stream = Filled(producer);
        Assert.Equal(NativeArrowStreamEnterResult.Entered, stream.TryEnter(out var lease));
        using var handle = new ArrowArrayHandle();

        using (lease)
        {
            var response = await driver.ArrowStreamGetNextBatchAsync(
                new ArrowStreamGetNextBatchRequest { Stream = lease!.ToProto(), OutArray = handle.ToProto() }, CancellationToken.None).ConfigureAwait(false);

            Assert.Equal(ProducerFailureStatus, response.Status);
            Assert.Contains("Failed to read chunk data", response.ErrorMessage, StringComparison.Ordinal);
        }

        Assert.True(handle.IsReleased);
    }

    internal static void ClearCallback(NativeArrowStream stream, string field) =>
        Marshal.WriteIntPtr(stream.DangerousGetHandle(), Marshal.OffsetOf<ArrowArrayStreamAbi>(field).ToInt32(), PtrHelper.Zero);

    internal static NativeArrowStream Filled(FakeArrowArrayStream producer) => Filled(producer, NullLogger.Instance);

    internal static NativeArrowStream Filled(FakeArrowArrayStream producer, ILogger logger)
    {
        var stream = new NativeArrowStream(logger);
        ArrowStreamExporter.ExportStreamInto(producer, stream.ToProto().Value.Span);
        return stream;
    }
}
