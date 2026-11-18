using System.Buffers.Binary;
using Apache.Arrow;
using Apache.Arrow.C;
using Apache.Arrow.Ipc;
using Snowflake.Data.Interop;

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
    public static nint Export(Schema schema, params RecordBatch[] batches) =>
        Export(new RecordBatchStream(schema, batches));

    /// <summary>Exports <paramref name="stream"/> and returns the address of the exported stream.</summary>
    /// <remarks>Releasing the exported stream disposes <paramref name="stream"/>.</remarks>
    public static unsafe nint Export(IArrowArrayStream stream)
    {
        var exported = CArrowArrayStream.Create();
        CArrowArrayStreamExporter.ExportArrayStream(stream, exported);
        return (nint)exported;
    }

    /// <summary>
    /// Exports <paramref name="stream"/> into the caller-allocated <c>struct ArrowArrayStream</c> at
    /// <paramref name="address"/> (8 little-endian bytes), as sf_core does for ResultSetGetStream with an out_stream.
    /// </summary>
    /// <remarks>Releasing the exported stream disposes <paramref name="stream"/>.</remarks>
    public static unsafe void ExportStreamInto(IArrowArrayStream stream, ReadOnlySpan<byte> address) =>
        CArrowArrayStreamExporter.ExportArrayStream(stream, (CArrowArrayStream*)BinaryPrimitives.ReadInt64LittleEndian(address));

    /// <summary>
    /// Releases the live stream at <paramref name="address"/> (8 little-endian bytes) and leaves the struct allocated,
    /// in the released state.
    /// </summary>
    /// <remarks>
    /// Importing moves the stream out of the struct, which reads its schema first; disposing the import calls the
    /// producer's <c>release</c>.
    /// </remarks>
    public static unsafe void ReleaseStreamAt(ReadOnlySpan<byte> address) =>
        CArrowArrayStreamImporter.ImportArrayStream((CArrowArrayStream*)BinaryPrimitives.ReadInt64LittleEndian(address)).Dispose();

    /// <summary>Whether the stream struct at <paramref name="address"/> is in the released state, with a null <c>release</c>.</summary>
    public static unsafe bool IsReleased(nint address) => ((ArrowArrayStreamAbi*)address)->Release == 0;

    /// <summary>Frees the stream struct at <paramref name="address"/>, which <see cref="Export(IArrowArrayStream)"/> allocated.</summary>
    public static unsafe void Free(nint address) => CArrowArrayStream.Free((CArrowArrayStream*)address);

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
