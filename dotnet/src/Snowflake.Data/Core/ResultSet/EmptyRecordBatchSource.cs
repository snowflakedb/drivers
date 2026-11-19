using Apache.Arrow;
using Snowflake.Data.Interfaces;

namespace Snowflake.Data.Core.ResultSet;

/// <summary>
/// A source with no batches and nothing to release, used for a described query, which has columns but no rows.
/// </summary>
internal sealed class EmptyRecordBatchSource : IRecordBatchSource
{
    private static readonly Task<RecordBatch?> NoBatch = Task.FromResult<RecordBatch?>(null);

    /// <summary>The shared stateless instance.</summary>
    internal static IRecordBatchSource Instance { get; } = new EmptyRecordBatchSource();

    public RecordBatch? Next() => null;

    public Task<RecordBatch?> NextAsync(CancellationToken cancellationToken) => NoBatch;

    public void Dispose()
    {
    }
}
