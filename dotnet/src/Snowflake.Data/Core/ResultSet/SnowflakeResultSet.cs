using System.Diagnostics;
using Apache.Arrow;
using Microsoft.Extensions.Logging;
using Snowflake.Data.Core.DataConversion.ResultSetConversion;
using Snowflake.Data.Core.Errors;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Proto;
using Snowflake.Data.Telemetry;

namespace Snowflake.Data.Core.ResultSet;

/// <summary>
/// Reads the rows of one sf_core result forward, batch by batch, and converts column values through one converter
/// per column.
/// </summary>
/// <remarks>
/// Rows arrive in batches fetched as <see cref="Read"/> and <see cref="ReadNextBatchAsync"/> reach them; moving within a
/// batch does no I/O, and empty batches are skipped. Every carrier thrown carries the result's query ID. A failed
/// fetch is logged and recorded on the fetch activity; a failed conversion is not logged, because it is the
/// caller's to handle. A lock serializes <see cref="Close"/> with cursor moves and column reads, so a batch is
/// never released while a value is being read from it; the fetch itself runs outside the lock, and a batch
/// fetched while the result set closes is disposed.
/// </remarks>
internal sealed class SnowflakeResultSet : ISnowflakeResultSet
{
    private readonly IResultSetInterpreter _interpreter;
    private readonly IRecordBatchSource _batches;
    private readonly ResultSetDescriptor _descriptor;
    private readonly ColumnConverter[] _converters;
    private readonly ILogger<SnowflakeResultSet> _logger;
    private readonly LockObject _gate = new();

    private RecordBatch? _batch;
    private RecordBatch? _peeked;
    private int _row = -1;
    private bool _sawRows;
    private bool _exhausted;
    private bool _closed;

    /// <param name="interpreter">Resolves the column names passed to <see cref="GetOrdinal"/> to ordinals.</param>
    /// <param name="batches">The result's batches; the result set takes ownership and disposes it on <see cref="Close"/>.</param>
    /// <param name="converterFactory">Chooses the converter for each column of <paramref name="descriptor"/>.</param>
    /// <param name="descriptor">The column metadata, query ID and row count of the result.</param>
    /// <param name="recordsAffected">The value reported as <see cref="RecordsAffected"/>.</param>
    /// <param name="logger">Receives the diagnostics of fetching batches.</param>
    internal SnowflakeResultSet(
        IResultSetInterpreter interpreter,
        IRecordBatchSource batches,
        IColumnConverterFactory converterFactory,
        ResultSetDescriptor descriptor,
        int recordsAffected,
        ILogger<SnowflakeResultSet> logger)
    {
        _interpreter = interpreter;
        _batches = batches;
        _descriptor = descriptor;
        RecordsAffected = recordsAffected;
        _logger = logger;
        _converters = Enumerable.Range(0, descriptor.Columns.Count).Select(x => converterFactory.Create(descriptor.Columns[x])).ToArray();
    }

    public string QueryId => _descriptor.QueryId;

    public int RecordsAffected { get; }

    public int FieldCount => _descriptor.Columns.Count;

    public bool IsClosed
    {
        get
        {
            lock (_gate)
                return _closed;
        }
    }

    private string? QueryIdOrNull => string.IsNullOrEmpty(_descriptor.QueryId) ? null : _descriptor.QueryId;

    public bool GetHasRows()
    {
        lock (_gate)
        {
            if (_closed)
                return false;

            if (_descriptor.HasRowCount)
                return _descriptor.RowCount > 0;

            if (_sawRows || _exhausted)
                return _sawRows;
        }

        PeekFirstRows();
        lock (_gate)
            return _sawRows;
    }

    public bool Read()
    {
        RecordBatch? peeked;
        lock (_gate)
        {
            ThrowIfClosed();
            if (TryAdvanceWithinBatchLocked())
                return true;

            peeked = TakePeeked();
        }

        while (true)
        {
            var next = peeked ?? FetchNext();
            peeked = null;
            lock (_gate)
            {
                DisposeIfClosed(next);
                var moved = MoveTo(next);
                if (moved.HasValue)
                    return moved.Value;
            }
        }
    }

    public bool TryAdvanceWithinBatch()
    {
        lock (_gate)
            return TryAdvanceWithinBatchLocked();
    }

    public async Task<bool> ReadNextBatchAsync(CancellationToken cancellationToken)
    {
        RecordBatch? peeked;
        lock (_gate)
        {
            ThrowIfClosed();
            peeked = TakePeeked();
        }

        while (true)
        {
            var next = peeked ?? await FetchNextAsync(cancellationToken).ConfigureAwait(false);
            peeked = null;
            lock (_gate)
            {
                DisposeIfClosed(next);
                var moved = MoveTo(next);
                if (moved.HasValue)
                    return moved.Value;
            }
        }
    }

    public int GetOrdinal(string name) =>
         _interpreter.TryGetOrdinal(_descriptor, name, out var ordinal)
            ? ordinal
            : throw new SnowflakeDriverException(SFError.COLUMN_INDEX_OUT_OF_BOUND, QueryIdOrNull, name);

    public string GetName(int ordinal)
    {
        ThrowIfOutOfRange(ordinal);
        return _descriptor.Columns[ordinal].Name;
    }

    public string GetDataTypeName(int ordinal)
    {
        ThrowIfOutOfRange(ordinal);
        return _descriptor.Columns[ordinal].Type.ToUpperInvariant();
    }

    public Type GetFieldType(int ordinal)
    {
        ThrowIfOutOfRange(ordinal);
        return _converters[ordinal].FieldType;
    }

    public bool IsDBNull(int ordinal) => ReadColumn(ordinal, static (column, row) => column.IsNull(row));

    public object GetValue(int ordinal) => ReadColumn(ordinal, static (column, row) => column.GetValue(row));

    public T? GetFieldValue<T>(int ordinal) => ReadColumn(ordinal, static (column, row) => column.Get<T>(row));

    public long GetBytes(int ordinal, long dataOffset, byte[]? buffer, int bufferOffset, int length) =>
        ReadColumn(ordinal, (column, row) => column.GetBytes(row, dataOffset, buffer, bufferOffset, length));

    public long GetChars(int ordinal, long dataOffset, char[]? buffer, int bufferOffset, int length) =>
        ReadColumn(ordinal, (column, row) => column.GetChars(row, dataOffset, buffer, bufferOffset, length));

    public void Close()
    {
        lock (_gate)
        {
            if (_closed)
                return;

            _closed = true;
            _logger.LogDebug("Closing the result of query {QueryId}", _descriptor.QueryId);
            DisposeBatch();
            _peeked?.Dispose();
            _peeked = null;
        }

        _batches.Dispose();
    }

    public void Dispose() => Close();

    /// <summary>
    /// Makes <paramref name="next"/> the current batch. Returns false at the end of the rows, true when
    /// positioned on the batch's first row, and null for an empty batch, which the caller skips.
    /// </summary>
    private bool? MoveTo(RecordBatch? next)
    {
        DisposeBatch();
        if (next is null)
        {
            _exhausted = true;
            return false;
        }

        if (next.Length == 0)
        {
            next.Dispose();
            return null;
        }

        Bind(next);
        return true;
    }

    private void Bind(RecordBatch batch)
    {
        try
        {
            if (batch.ColumnCount != _converters.Length)
            {
                throw new SnowflakeDriverException(SFError.INTERNAL_ERROR, QueryIdOrNull,
                    $"A batch has {batch.ColumnCount} columns but the result has {_converters.Length}.");
            }

            for (var i = 0; i < _converters.Length; i++)
                _converters[i].Bind(batch.Column(i));
        }
        catch (Exception ex)
        {
            batch.Dispose();
            if (ex is SnowflakeException carrier)
                carrier.QueryId ??= QueryIdOrNull;

            throw;
        }

        _batch = batch;
        _row = 0;
        _sawRows = true;
    }

    private void PeekFirstRows()
    {
        while (true)
        {
            lock (_gate)
            {
                if (_closed || _peeked is not null || _exhausted)
                    return;
            }

            var next = FetchNext();
            lock (_gate)
            {
                if (_closed)
                {
                    next?.Dispose();
                    return;
                }

                if (next is null)
                {
                    _exhausted = true;
                    return;
                }

                if (next.Length == 0)
                {
                    next.Dispose();
                    continue;
                }

                _peeked = next;
                _sawRows = true;
                return;
            }
        }
    }

    private RecordBatch? TakePeeked()
    {
        var peeked = _peeked;
        _peeked = null;
        return peeked;
    }

    private RecordBatch? FetchNext()
    {
        if (_exhausted)
            return null;

        _logger.LogDebug("Fetching the next batch of query {QueryId}", _descriptor.QueryId);
        using var activity = LoggerAndTelemetryStubs.StartActivity(TelemetryActivities.DataReaderFetchBatch);
        try
        {
            var batch = _batches.Next();
            activity?.SetSuccess();
            return batch;
        }
        catch (SnowflakeException carrier)
        {
            RecordFetchFailure(carrier, activity);
            throw;
        }
    }

    private async Task<RecordBatch?> FetchNextAsync(CancellationToken cancellationToken)
    {
        if (_exhausted)
            return null;

        _logger.LogDebug("Fetching the next batch of query {QueryId}", _descriptor.QueryId);
        using var activity = LoggerAndTelemetryStubs.StartActivity(TelemetryActivities.DataReaderFetchBatchAsync);
        try
        {
            var batch = await _batches.NextAsync(cancellationToken).ConfigureAwait(false);
            activity?.SetSuccess();
            return batch;
        }
        catch (SnowflakeException carrier)
        {
            RecordFetchFailure(carrier, activity);
            throw;
        }
    }

    private void RecordFetchFailure(SnowflakeException carrier, Activity? activity)
    {
        carrier.QueryId ??= QueryIdOrNull;
        if (carrier is SnowflakeCancellationException)
            _logger.LogDebug("Fetching the next batch of query {QueryId} was cancelled.", _descriptor.QueryId);
        else
            _logger.LogError(carrier, "Fetching the next batch of query {QueryId} failed.", _descriptor.QueryId);

        activity?.SetException(carrier);
    }

    private T ReadColumn<T>(int ordinal, Func<ColumnConverter, int, T> read)
    {
        lock (_gate)
        {
            var column = CurrentColumn(ordinal);
            try
            {
                return read(column, _row);
            }
            catch (SnowflakeException carrier)
            {
                carrier.QueryId ??= QueryIdOrNull;
                throw;
            }
        }
    }

    private bool TryAdvanceWithinBatchLocked()
    {
        ThrowIfClosed();
        if (_batch is null || _row + 1 >= _batch.Length)
            return false;

        _row++;
        return true;
    }

    private void DisposeIfClosed(RecordBatch? fetched)
    {
        if (!_closed)
            return;

        fetched?.Dispose();
        ThrowIfClosed();
    }

    private ColumnConverter CurrentColumn(int ordinal)
    {
        ThrowIfClosed();
        ThrowIfOutOfRange(ordinal);
        if (_batch is null)
            throw new SnowflakeDriverException(SFError.DATA_READER_NO_CURRENT_ROW, QueryIdOrNull);

        return _converters[ordinal];
    }

    private void ThrowIfClosed()
    {
        if (_closed)
            throw new SnowflakeDriverException(SFError.DATA_READER_ALREADY_CLOSED, QueryIdOrNull);
    }

    private void ThrowIfOutOfRange(int ordinal)
    {
        if ((uint)ordinal >= (uint)_converters.Length)
            throw new SnowflakeDriverException(SFError.COLUMN_INDEX_OUT_OF_BOUND, QueryIdOrNull, ordinal);
    }

    private void DisposeBatch()
    {
        _batch?.Dispose();
        _batch = null;
        _row = -1;
        foreach (var converter in _converters)
            converter.Unbind();
    }
}
