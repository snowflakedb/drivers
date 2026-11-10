using Apache.Arrow;
using Apache.Arrow.C;
using Apache.Arrow.Ipc;

namespace Snowflake.Data.Tests.Utilities;

/// <summary>
/// Exports record batches as an Arrow C stream, the form in which sf_core hands a result to the driver.
/// </summary>
/// <remarks>
/// Importing the stream moves it out of the exported struct, and disposing the imported stream releases the
/// batches. The struct itself, allocated by <see cref="CArrowArrayStream.Create"/>, is not freed.
/// </remarks>
public static class ArrowStreamExporter
{
    /// <summary>Exports <paramref name="batches"/>, which all have <paramref name="schema"/>, and returns the address of the exported stream.</summary>
    public static unsafe nint Export(Schema schema, params RecordBatch[] batches)
    {
        var stream = CArrowArrayStream.Create();
        CArrowArrayStreamExporter.ExportArrayStream(new RecordBatchStream(schema, batches), stream);
        return (nint)stream;
    }

    private sealed class RecordBatchStream : IArrowArrayStream
    {
        private readonly RecordBatch[] _batches;
        private int _next;

        internal RecordBatchStream(Schema schema, RecordBatch[] batches)
        {
            Schema = schema;
            _batches = batches;
        }

        public Schema Schema { get; }

        public ValueTask<RecordBatch> ReadNextRecordBatchAsync(CancellationToken cancellationToken = default) =>
            new(_next < _batches.Length ? _batches[_next++] : null!);

        public void Dispose()
        {
            foreach (var batch in _batches)
                batch.Dispose();
        }
    }
}
