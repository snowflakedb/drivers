using Apache.Arrow;
using Google.Protobuf;
using NSubstitute;
using NSubstitute.ExceptionExtensions;
using Snowflake.Data.Core;
using Snowflake.Data.Core.Errors;
using Snowflake.Data.Core.ResultSet;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Tests.Core;

[Trait("Category", "Unit")]
public sealed class SfCoreRecordBatchSourceTest : IDisposable
{
    private const string QueryId = "01bf4a3e-0001-2c4f-0000-00a1b2c3d4e5";

    private readonly IDatabaseDriverService _driver = Substitute.For<IDatabaseDriverService>();
    private readonly RecordBatch _first = UnitBatches.Batch(UnitBatches.Int64(1, 2));
    private readonly RecordBatch _second = UnitBatches.Batch(UnitBatches.Int64(3));
    private readonly FakeArrowArrayStream _producer;
    private readonly SfCoreRecordBatchSource _source;

    public SfCoreRecordBatchSourceTest()
    {
        _producer = new FakeArrowArrayStream(_first.Schema, _first, _second);
        _source = SfCoreRecordBatchSource.Open(_driver, NativeArrowStreamTest.Filled(_producer), QueryId);
        UnitStreamRpcs.ServeGetNext(_driver);
    }

    public void Dispose()
    {
        _producer.Gate?.Release(int.MaxValue / 2);
        _source.Dispose();
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void NextReadsTheStreamDirectlyThenReturnsNullAndReleasesTheStreamAtTheEnd()
    {
        using var first = _source.Next();
        using var second = _source.Next();
        Assert.Equal(0, _producer.DisposeCalls);
        var end = _source.Next();

        Assert.Equal([1L, 2L], Int64Values(first));
        Assert.Equal([3L], Int64Values(second));
        Assert.Null(end);
        Assert.Equal(1, _producer.DisposeCalls);
        Assert.Empty(_driver.ReceivedCalls());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task NextAsyncHasSfCoreReadTheStreamThenReturnsNullAndReleasesTheStreamAtTheEndAsync()
    {
        using var cancellation = new CancellationTokenSource();

        using var first = await _source.NextAsync(cancellation.Token).ConfigureAwait(false);
        using var second = await _source.NextAsync(cancellation.Token).ConfigureAwait(false);
        var end = await _source.NextAsync(cancellation.Token).ConfigureAwait(false);

        Assert.Equal([1L, 2L], Int64Values(first));
        Assert.Equal([3L], Int64Values(second));
        Assert.Null(end);
        Assert.Equal(1, _producer.DisposeCalls);
        _ = _driver.Received(3).ArrowStreamGetNextBatchAsync(
            Arg.Is<ArrowStreamGetNextBatchRequest>(request => request.Stream.Value.Length == sizeof(long) && request.OutArray.Value.Length == sizeof(long)),
            cancellation.Token);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task AfterTheEndNothingMoreIsReadAsync()
    {
        _source.Next();
        _source.Next();
        Assert.Null(_source.Next());

        Assert.Null(_source.Next());
        Assert.Null(await _source.NextAsync(CancellationToken.None).ConfigureAwait(false));

        Assert.Empty(_driver.ReceivedCalls());
        _source.Dispose();
        AssertDriverError(SFError.DATA_READER_ALREADY_CLOSED, Assert.Throws<SnowflakeDriverException>(() => _source.Next()));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task ReadingAfterDisposeIsAReaderClosedCarrierAsync()
    {
        _source.Dispose();
        _source.Dispose();

        Assert.Equal(1, _producer.DisposeCalls);
        AssertDriverError(SFError.DATA_READER_ALREADY_CLOSED, Assert.Throws<SnowflakeDriverException>(() => _source.Next()));
        AssertDriverError(
            SFError.DATA_READER_ALREADY_CLOSED,
            await Assert.ThrowsAsync<SnowflakeDriverException>(() => _source.NextAsync(CancellationToken.None)).ConfigureAwait(false));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(false)]
    [InlineData(true)]
    public async Task AFailedReadIsAFetchFailedCarrierWithTheStreamTextAndStatusAsync(bool async)
    {
        _producer.FailNextWith = new InvalidOperationException("Failed to read chunk data");

        var exception = await ReadExpectingFailureAsync(async, CancellationToken.None).ConfigureAwait(false);

        AssertDriverError(SFError.QUERY_RESULT_FETCH_FAILED, exception);
        Assert.Contains("Failed to read chunk data", Assert.IsType<string>(exception.Args[0]), StringComparison.Ordinal);
        Assert.Equal(NativeArrowStreamTest.ProducerFailureStatus, Assert.IsType<int>(exception.Args[1]));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(false)]
    [InlineData(true)]
    public async Task FailedReadReleasesTheStreamAndEveryLaterReadThrowsTheSameFailureAsync(bool failAsync)
    {
        _producer.FailNextWith = new InvalidOperationException("Failed to read chunk data");
        var failure = await ReadExpectingFailureAsync(failAsync, CancellationToken.None).ConfigureAwait(false);
        Assert.Equal(1, _producer.DisposeCalls);
        _driver.ClearReceivedCalls();

        var laterReads = new[]
        {
            Assert.Throws<SnowflakeDriverException>(() => _source.Next()),
            await Assert.ThrowsAsync<SnowflakeDriverException>(() => _source.NextAsync(CancellationToken.None)).ConfigureAwait(false),
            Assert.Throws<SnowflakeDriverException>(() => _source.Next()),
        };

        foreach (var exception in laterReads)
        {
            AssertDriverError(SFError.QUERY_RESULT_FETCH_FAILED, exception);
            Assert.Same(failure, exception);
            Assert.Equal(failure.Args, exception.Args);
        }
        Assert.Empty(_driver.ReceivedCalls());
        _source.Dispose();
        Assert.Equal(1, _producer.DisposeCalls);
        AssertDriverError(SFError.DATA_READER_ALREADY_CLOSED, Assert.Throws<SnowflakeDriverException>(() => _source.Next()));
        AssertDriverError(
            SFError.DATA_READER_ALREADY_CLOSED,
            await Assert.ThrowsAsync<SnowflakeDriverException>(() => _source.NextAsync(CancellationToken.None)).ConfigureAwait(false));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task AnSfCoreErrorBecomesACarrierWithTheQueryIdAsync()
    {
        _driver.ArrowStreamGetNextBatchAsync(Arg.Any<ArrowStreamGetNextBatchRequest>(), Arg.Any<CancellationToken>())
            .ThrowsAsync(new ServiceException(new DriverException { Message = "stream is released", Kind = ErrorKind.InvalidArgument }));

        var exception = await Assert.ThrowsAsync<SnowflakeDriverException>(() => _source.NextAsync(CancellationToken.None)).ConfigureAwait(false);

        AssertDriverError(SFError.INTERNAL_ERROR, exception);
        Assert.Equal("stream is released", Assert.Single(exception.Args));
        await AssertLaterReadsRethrowWithoutReadingAsync(exception, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task ATransportFailureBecomesAnInternalErrorCarrierAsync()
    {
        _driver.ArrowStreamGetNextBatchAsync(Arg.Any<ArrowStreamGetNextBatchRequest>(), Arg.Any<CancellationToken>())
            .ThrowsAsync(new TransportException("transport down"));

        var exception = await Assert.ThrowsAsync<SnowflakeDriverException>(() => _source.NextAsync(CancellationToken.None)).ConfigureAwait(false);

        AssertDriverError(SFError.INTERNAL_ERROR, exception);
        Assert.Equal("transport down", Assert.Single(exception.Args));
        await AssertLaterReadsRethrowWithoutReadingAsync(exception, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task AnAlreadyCancelledTokenIsAQueryCancelledCarrierAndReadsNothingAsync()
    {
        using var cancellation = new CancellationTokenSource();
        cancellation.Cancel();

        var exception = await Assert.ThrowsAsync<SnowflakeCancellationException>(() => _source.NextAsync(cancellation.Token)).ConfigureAwait(false);

        Assert.Equal(SFError.QUERY_CANCELLED, exception.Error);
        Assert.Equal(QueryId, exception.QueryId);
        Assert.Empty(_driver.ReceivedCalls());
        using var first = _source.Next();
        Assert.Equal([1L, 2L], Int64Values(first));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task ATransportThatRejectsTheCancelledTokenIsAQueryCancelledCarrierAsync()
    {
        using var cancellation = new CancellationTokenSource();
        var cancelled = new OperationCanceledException(cancellation.Token);
        _driver.ArrowStreamGetNextBatchAsync(Arg.Any<ArrowStreamGetNextBatchRequest>(), Arg.Any<CancellationToken>())
            .Returns(_ =>
            {
                cancellation.Cancel();
                return Task.FromException<ArrowStreamGetNextBatchResponse>(cancelled);
            });

        var exception = await Assert.ThrowsAsync<SnowflakeCancellationException>(() => _source.NextAsync(cancellation.Token)).ConfigureAwait(false);

        Assert.Equal(SFError.QUERY_CANCELLED, exception.Error);
        Assert.Same(cancelled, exception.InnerException);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task CancellingDispatchedReadWaitsForSfCoreToFillSlotAndReturnsBatchAsync()
    {
        var transport = Substitute.For<ICoreTransport>();
        var transportToken = new TaskCompletionSource<CancellationToken>(TaskCreationOptions.RunContinuationsAsynchronously);
        transport.HandleMessageAsync("DatabaseDriver", "arrow_stream_get_next_batch", Arg.Any<byte[]>(), Arg.Any<CancellationToken>())
            .Returns(call =>
            {
                transportToken.TrySetResult(call.Arg<CancellationToken>());
                var request = ArrowStreamGetNextBatchRequest.Parser.ParseFrom(call.Arg<byte[]>());
                return Task.Run(() => Success(UnitStreamRpcs.GetNext(request)));
            });
        var client = new DatabaseDriverServiceClient(transport);
        _driver.ArrowStreamGetNextBatchAsync(Arg.Any<ArrowStreamGetNextBatchRequest>(), Arg.Any<CancellationToken>())
            .Returns(call => client.ArrowStreamGetNextBatchAsync(call.Arg<ArrowStreamGetNextBatchRequest>(), call.Arg<CancellationToken>()));
        _producer.Gate = new SemaphoreSlim(0);
        using var cancellation = new CancellationTokenSource();

        var inFlight = _source.NextAsync(cancellation.Token);
        Assert.True(_producer.ReadStarted.Wait(TimeSpan.FromSeconds(30)));
        cancellation.Cancel();

        Assert.False((await transportToken.Task.ConfigureAwait(false)).CanBeCanceled);
        Assert.False(inFlight.IsCompleted);
        _producer.Gate.Release();
        using var first = await inFlight.ConfigureAwait(false);
        Assert.Equal([1L, 2L], Int64Values(first));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task AReadWhileAnotherIsInProgressIsAFetchInProgressCarrierAndTouchesNothingAsync()
    {
        _producer.Gate = new SemaphoreSlim(0);
        var inFlight = _source.NextAsync(CancellationToken.None);
        Assert.True(_producer.ReadStarted.Wait(TimeSpan.FromSeconds(30)));

        AssertDriverError(SFError.DATA_READER_FETCH_IN_PROGRESS, Assert.Throws<SnowflakeDriverException>(() => _source.Next()));
        AssertDriverError(
            SFError.DATA_READER_FETCH_IN_PROGRESS,
            await Assert.ThrowsAsync<SnowflakeDriverException>(() => _source.NextAsync(CancellationToken.None)).ConfigureAwait(false));
        _ = _driver.Received(1).ArrowStreamGetNextBatchAsync(Arg.Any<ArrowStreamGetNextBatchRequest>(), Arg.Any<CancellationToken>());

        _producer.Gate.Release();
        using var first = await inFlight.ConfigureAwait(false);
        Assert.Equal([1L, 2L], Int64Values(first));
        _producer.Gate = null;
        using var second = _source.Next();
        Assert.Equal([3L], Int64Values(second));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task DisposingDuringAReadReleasesTheStreamWhenTheReadEndsAndTheReadReportsTheReaderClosedAsync()
    {
        _producer.Gate = new SemaphoreSlim(0);
        var inFlight = _source.NextAsync(CancellationToken.None);
        Assert.True(_producer.ReadStarted.Wait(TimeSpan.FromSeconds(30)));

        _source.Dispose();
        Assert.Equal(0, _producer.DisposeCalls);
        _producer.Gate.Release();

        AssertDriverError(
            SFError.DATA_READER_ALREADY_CLOSED,
            await Assert.ThrowsAsync<SnowflakeDriverException>(() => inFlight).ConfigureAwait(false));
        Assert.Equal(1, _producer.DisposeCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void OpenWithAFailedSchemaIsAFetchFailedCarrierAndDisposesTheStream()
    {
        var producer = new FakeArrowArrayStream(_first.Schema) { FailSchemaWith = new InvalidOperationException("Failed to read schema") };

        var exception = Assert.Throws<SnowflakeDriverException>(() => SfCoreRecordBatchSource.Open(_driver, NativeArrowStreamTest.Filled(producer), QueryId));

        AssertDriverError(SFError.QUERY_RESULT_FETCH_FAILED, exception);
        Assert.Contains("Failed to read schema", Assert.IsType<string>(exception.Args[0]), StringComparison.Ordinal);
        Assert.Equal(NativeArrowStreamTest.ProducerFailureStatus, Assert.IsType<int>(exception.Args[1]));
        Assert.Equal(1, producer.DisposeCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task CancellationNotRequestedByTheCallerPropagatesAsItIsAsync()
    {
        using var cancellation = new CancellationTokenSource();
        var cancelled = new OperationCanceledException("not the caller's token");
        _driver.ArrowStreamGetNextBatchAsync(Arg.Any<ArrowStreamGetNextBatchRequest>(), Arg.Any<CancellationToken>())
            .Returns(Task.FromException<ArrowStreamGetNextBatchResponse>(cancelled));

        var exception = await Assert.ThrowsAsync<OperationCanceledException>(() => _source.NextAsync(cancellation.Token)).ConfigureAwait(false);

        Assert.Same(cancelled, exception);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task FailureStatusWithoutAMessageIsAFetchFailedCarrierWithAnEmptyTextAsync()
    {
        _driver.ArrowStreamGetNextBatchAsync(Arg.Any<ArrowStreamGetNextBatchRequest>(), Arg.Any<CancellationToken>())
            .Returns(new ArrowStreamGetNextBatchResponse { Status = 5 });

        var exception = await Assert.ThrowsAsync<SnowflakeDriverException>(() => _source.NextAsync(CancellationToken.None)).ConfigureAwait(false);

        AssertDriverError(SFError.QUERY_RESULT_FETCH_FAILED, exception);
        Assert.Equal(string.Empty, exception.Args[0]);
        Assert.Equal(5, exception.Args[1]);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task DisposingDuringASynchronousReadReleasesTheStreamWhenTheReadEndsAndTheReadReportsTheReaderClosedAsync()
    {
        _producer.Gate = new SemaphoreSlim(0);
        var inFlight = Task.Run(() => _source.Next());
        Assert.True(_producer.ReadStarted.Wait(TimeSpan.FromSeconds(30)));

        _source.Dispose();
        Assert.Equal(0, _producer.DisposeCalls);
        _producer.Gate.Release();

        AssertDriverError(
            SFError.DATA_READER_ALREADY_CLOSED,
            await Assert.ThrowsAsync<SnowflakeDriverException>(() => inFlight).ConfigureAwait(false));
        Assert.Equal(1, _producer.DisposeCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task CancellationBeforeDispatchLeavesSourceReadableLikeAlreadyCancelledTokenAsync()
    {
        var transport = Substitute.For<ICoreTransport>();
        var client = new DatabaseDriverServiceClient(transport);
        using var cancellation = new CancellationTokenSource();
        _driver.ArrowStreamGetNextBatchAsync(Arg.Any<ArrowStreamGetNextBatchRequest>(), Arg.Any<CancellationToken>())
            .Returns(call =>
            {
                cancellation.Cancel();
                return client.ArrowStreamGetNextBatchAsync(call.Arg<ArrowStreamGetNextBatchRequest>(), call.Arg<CancellationToken>());
            });

        var exception = await Assert.ThrowsAsync<SnowflakeCancellationException>(() => _source.NextAsync(cancellation.Token)).ConfigureAwait(false);

        Assert.Equal(SFError.QUERY_CANCELLED, exception.Error);
        Assert.Empty(transport.ReceivedCalls());
        Assert.False(_producer.ReadStarted.IsSet);
        Assert.Equal(0, _producer.DisposeCalls);
        using var first = _source.Next();
        Assert.Equal([1L, 2L], Int64Values(first));
    }

    private async Task<SnowflakeDriverException> ReadExpectingFailureAsync(bool async, CancellationToken cancellationToken) =>
        async
            ? await Assert.ThrowsAsync<SnowflakeDriverException>(() => _source.NextAsync(cancellationToken)).ConfigureAwait(false)
            : Assert.Throws<SnowflakeDriverException>(() => _source.Next());

    private async Task AssertLaterReadsRethrowWithoutReadingAsync(SnowflakeDriverException failure, CancellationToken cancellationToken)
    {
        Assert.Equal(1, _producer.DisposeCalls);
        _driver.ClearReceivedCalls();

        Assert.Same(failure, Assert.Throws<SnowflakeDriverException>(() => _source.Next()));
        Assert.Same(
            failure,
            await Assert.ThrowsAsync<SnowflakeDriverException>(() => _source.NextAsync(CancellationToken.None)).ConfigureAwait(false));

        Assert.Empty(_driver.ReceivedCalls());
        Assert.False(_producer.ReadStarted.IsSet);
    }

    private static void AssertDriverError(SFError expected, SnowflakeDriverException exception)
    {
        Assert.Equal(expected, exception.Error);
        Assert.Equal(QueryId, exception.QueryId);
    }

    private static TransportResponse Success(IMessage response) =>
        new(TransportResponse.CodeSuccess, new ArraySegment<byte>(response.ToByteArray()), null!);

    private static long[] Int64Values(RecordBatch? batch)
    {
        Assert.NotNull(batch);
        var column = Assert.IsType<Int64Array>(batch.Column(0));
        return Enumerable.Range(0, column.Length).Select(i => column.GetValue(i)!.Value).ToArray();
    }
}
