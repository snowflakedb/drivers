using Apache.Arrow;
using Snowflake.Data.Core.Errors;
using Snowflake.Data.Interfaces;

namespace Snowflake.Data.Tests.Core;

/// <summary>
/// An <see cref="IRecordBatchSource"/> over fixed batches that records how it is used.
/// </summary>
internal sealed class FakeRecordBatchSource : IRecordBatchSource
{
    private readonly Queue<RecordBatch> _batches;

    internal FakeRecordBatchSource(params RecordBatch[] batches)
    {
        _batches = new Queue<RecordBatch>(batches);
    }

    /// <summary>Thrown by the next fetch instead of returning a batch, when set.</summary>
    internal Exception? FailNextWith { get; set; }

    internal int NextCalls { get; private set; }

    internal int NextAsyncCalls { get; private set; }

    internal int DisposeCalls { get; private set; }

    public RecordBatch? Next()
    {
        NextCalls++;
        return Take();
    }

    public Task<RecordBatch?> NextAsync(CancellationToken cancellationToken)
    {
        NextAsyncCalls++;
        if (cancellationToken.IsCancellationRequested)
            throw new SnowflakeCancellationException(SFError.QUERY_CANCELLED, null, null);

        return Task.FromResult(Take());
    }

    public void Dispose() => DisposeCalls++;

    private RecordBatch? Take()
    {
        if (FailNextWith is { } failure)
        {
            FailNextWith = null;
            throw failure;
        }

        return _batches.Count > 0 ? _batches.Dequeue() : null;
    }
}
