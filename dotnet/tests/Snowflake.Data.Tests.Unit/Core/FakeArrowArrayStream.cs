using Apache.Arrow;
using Apache.Arrow.Ipc;

namespace Snowflake.Data.Tests.Core;

/// <summary>
/// A managed Arrow stream over fixed batches that stands in for sf_core's chunk reader once exported as an
/// Arrow C stream: each read can be held at a gate, and a read can be made to fail.
/// </summary>
internal sealed class FakeArrowArrayStream : IArrowArrayStream
{
    private readonly Schema _schema;
    private readonly Queue<RecordBatch> _batches;
    private int _disposeCalls;

    internal FakeArrowArrayStream(Schema schema, params RecordBatch[] batches)
    {
        _schema = schema;
        _batches = new Queue<RecordBatch>(batches);
    }

    public Schema Schema => FailSchemaWith is { } failure ? throw failure : _schema;

    /// <summary>When set, each read waits for a release of this gate after signalling <see cref="ReadStarted"/>.</summary>
    internal SemaphoreSlim? Gate { get; set; }

    /// <summary>Set once a read has started.</summary>
    internal ManualResetEventSlim ReadStarted { get; } = new();

    /// <summary>Thrown whenever the schema is requested, when set.</summary>
    internal Exception? FailSchemaWith { get; set; }

    /// <summary>Thrown by the next read instead of returning a batch, when set.</summary>
    internal Exception? FailNextWith { get; set; }

    /// <summary>How many times the exported stream's release callback disposed this stream.</summary>
    internal int DisposeCalls => Volatile.Read(ref _disposeCalls);

    public ValueTask<RecordBatch> ReadNextRecordBatchAsync(CancellationToken cancellationToken = default)
    {
        ReadStarted.Set();
        if (Gate is { } gate && !gate.Wait(TimeSpan.FromSeconds(30)))
            throw new TimeoutException("The gate was not released while a read waited on it.");

        if (FailNextWith is { } failure)
        {
            FailNextWith = null;
            throw failure;
        }

        return new(_batches.Count > 0 ? _batches.Dequeue() : null!);
    }

    public void Dispose() => Interlocked.Increment(ref _disposeCalls);
}
