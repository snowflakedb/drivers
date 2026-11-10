using System.Collections;
using System.Data.Common;
using Apache.Arrow;
using Apache.Arrow.C;
using Apache.Arrow.Ipc;
using Snowflake.Data.Core.Handles;
using Snowflake.Data.Proto;

namespace Snowflake.Data;

// TODO SNOW-4382582 this implementation is just PoC and will undergo heavy refactoring.
public sealed class SnowflakeDbDataReader : DbDataReader
{
    private readonly NativeResultSetHandle? _resultSet;
    private readonly ResultSetDescriptor _descriptor;
    private readonly IArrowArrayStream _arrowStream;
    private readonly int _recordsAffected;
    private readonly bool _schemaOnly;

    private RecordBatch? _currentBatch;
    private int _rowIndexInBatch = -1;
    private bool _closed;
    private bool _exhausted;

    /// <summary>
    /// Opens a reader over an sf_core Arrow stream and takes ownership of <paramref name="resultSet"/>,
    /// which is released on close; a describe-only result has none, and reports no rows.
    /// </summary>
    internal unsafe SnowflakeDbDataReader(
        NativeResultSetHandle? resultSet,
        ArrowArrayStreamPtr arrowStreamPtr,
        ResultSetDescriptor descriptor,
        int recordsAffected,
        bool schemaOnly)
    {
        _resultSet = resultSet;
        _descriptor = descriptor;
        _recordsAffected = recordsAffected;
        _schemaOnly = schemaOnly;

        // Import the native Arrow C Data Interface stream pointer.
        var ptrBytes = arrowStreamPtr.Value.ToByteArray();
        var nativePtr = (nint)BitConverter.ToInt64(ptrBytes, 0);
        var cStream = (CArrowArrayStream*)nativePtr;
        _arrowStream = CArrowArrayStreamImporter.ImportArrayStream(cStream);
    }

    public override int FieldCount => _descriptor.Columns.Count;

    public override int RecordsAffected => _recordsAffected;

    public override bool HasRows => !_schemaOnly && (!_descriptor.HasRowsAffected || _descriptor.RowsAffected > 0);

    public override bool IsClosed => _closed;

    public override int Depth => 0;

    public override object this[int ordinal] => GetValue(ordinal);

    public override object this[string name] => GetValue(GetOrdinal(name));

    // TODO this implementation is just PoC and will undergo heavy refactoring.
    public override bool Read()
    {
        if (_closed || _exhausted)
            return false;

        // Try to advance within the current batch.
        if (_currentBatch is not null)
        {
            _rowIndexInBatch++;
            if (_rowIndexInBatch < _currentBatch.Length)
                return true;
        }

        // Current batch exhausted — load next batch.
        while (true)
        {
            _currentBatch = _arrowStream.ReadNextRecordBatchAsync().GetAwaiter().GetResult();
            if (_currentBatch is null)
            {
                _exhausted = true;
                return false;
            }

            if (_currentBatch.Length > 0)
            {
                _rowIndexInBatch = 0;
                return true;
            }
            // Skip empty batches.
        }
    }

    // TODO this implementation is just PoC and will undergo heavy refactoring.
    public override bool IsDBNull(int ordinal)
    {
        EnsurePositioned();
        var column = _currentBatch!.Column(ordinal);
        return column.IsNull(_rowIndexInBatch);
    }

    public override object GetValue(int ordinal)
    {
        if (IsDBNull(ordinal))
            return DBNull.Value;

        var column = _currentBatch!.Column(ordinal);
        return ExtractValue(column, _rowIndexInBatch);
    }

    // TODO this implementation is just PoC and will undergo heavy refactoring.
    public override int GetValues(object[] values)
    {
        var count = Math.Min(values.Length, FieldCount);
        for (var i = 0; i < count; i++)
            values[i] = GetValue(i);
        return count;
    }

    // TODO this implementation is just PoC and will undergo heavy refactoring.
    public override long GetInt64(int ordinal)
    {
        EnsurePositioned();
        var column = _currentBatch!.Column(ordinal);
        return ExtractInt64(column, _rowIndexInBatch);
    }

    // TODO this implementation is just PoC and will undergo heavy refactoring.
    public override int GetInt32(int ordinal)
    {
        EnsurePositioned();
        var column = _currentBatch!.Column(ordinal);
        return (int)ExtractInt64(column, _rowIndexInBatch);
    }

    // TODO this implementation is just PoC and will undergo heavy refactoring.
    public override short GetInt16(int ordinal)
    {
        EnsurePositioned();
        var column = _currentBatch!.Column(ordinal);
        return (short)ExtractInt64(column, _rowIndexInBatch);
    }

    // TODO this implementation is just PoC and will undergo heavy refactoring.
    public override decimal GetDecimal(int ordinal)
    {
        EnsurePositioned();
        var column = _currentBatch!.Column(ordinal);
        var scale = "0";
        _currentBatch.Schema.FieldsLookup.ElementAt(ordinal).FirstOrDefault()?.Metadata.TryGetValue("scale", out scale);
        var result = ExtractDecimal(column, _rowIndexInBatch);
        var scaleNum = Math.Pow(10, int.Parse(scale ?? "0"));
        result /= (decimal)scaleNum;
        return result;
    }

    // TODO: Implement remaining typed accessors.
    public override bool GetBoolean(int ordinal) => throw new NotImplementedException();

    public override byte GetByte(int ordinal) => throw new NotImplementedException();

    public override long GetBytes(int ordinal, long dataOffset, byte[]? buffer, int bufferOffset, int length) =>
        throw new NotImplementedException();

    public override char GetChar(int ordinal) => throw new NotImplementedException();

    public override long GetChars(int ordinal, long dataOffset, char[]? buffer, int bufferOffset, int length) =>
        throw new NotImplementedException();

    public override string GetDataTypeName(int ordinal) => _descriptor.Columns[ordinal].Type;

    // TODO this implementation is just PoC and will undergo heavy refactoring.
    public override DateTime GetDateTime(int ordinal)
    {
        EnsurePositioned();
        var column = _currentBatch!.Column(ordinal);

        // Fallback: convert value to string.
        var value = ExtractValue(column, _rowIndexInBatch);

        // nanoseconds
        if (value is long l)
            return new DateTime(1970, 1, 1) + TimeSpan.FromMilliseconds(l / 1_000_000);

        return (DateTime)value;
    }

    // TODO this implementation is just PoC and will undergo heavy refactoring.
    public override double GetDouble(int ordinal)
    {
        EnsurePositioned();
        var column = _currentBatch!.Column(ordinal);
        return (double)ExtractValue(column, _rowIndexInBatch);
    }

    // TODO this implementation is just PoC and will undergo heavy refactoring.
    public override Type GetFieldType(int ordinal)
    {
        var colType = _descriptor.Columns[ordinal].Type.ToUpperInvariant();
        var scale = _descriptor.Columns[ordinal].HasScale ? _descriptor.Columns[ordinal].Scale : 0;
        return colType switch
        {
            "FIXED" => scale == 0 ? typeof(long) : typeof(decimal),
            "REAL" => typeof(double),
            "TEXT" => typeof(string),
            "BOOLEAN" => typeof(bool),
            "BINARY" => typeof(byte[]),
            "DATE" => typeof(DateTime),
            _ => typeof(string),
        };
    }

    public override float GetFloat(int ordinal) => throw new NotImplementedException();

    public override Guid GetGuid(int ordinal) => throw new NotImplementedException();

    public override string GetName(int ordinal) => _descriptor.Columns[ordinal].Name;

    // TODO: this implementation is just PoC and will undergo heavy refactoring.
    public override int GetOrdinal(string name)
    {
        for (var i = 0; i < _descriptor.Columns.Count; i++)
        {
            if (string.Equals(_descriptor.Columns[i].Name, name, StringComparison.OrdinalIgnoreCase))
                return i;
        }

        throw new IndexOutOfRangeException($"Column '{name}' not found.");
    }

    // TODO this implementation is just PoC and will undergo heavy refactoring.
    public override string GetString(int ordinal)
    {
        EnsurePositioned();
        var column = _currentBatch!.Column(ordinal);
        if (column is StringArray strArr)
            return strArr.GetString(_rowIndexInBatch);

        // Fallback: convert value to string.
        var value = ExtractValue(column, _rowIndexInBatch);
        return value.ToString() ?? string.Empty;
    }

    public override bool NextResult() => false;

    public override IEnumerator GetEnumerator() => new DbEnumerator(this);

    // TODO: this implementation is just PoC and will undergo heavy refactoring.
    public override void Close()
    {
        if (_closed)
            return;

        _closed = true;
        _currentBatch = null;

        try
        {
            _arrowStream.Dispose();
        }
        catch
        {
            // Best-effort release of Arrow stream.
        }

        _resultSet?.Dispose();
    }

    // TODO: this implementation is just PoC and will undergo heavy refactoring.
    protected override void Dispose(bool disposing)
    {
        if (disposing)
            Close();

        base.Dispose(disposing);
    }

    // TODO this is just a PoC and will be changed significantly
    private void EnsurePositioned()
    {
        if (_currentBatch is null || _rowIndexInBatch < 0)
            throw new InvalidOperationException("No current row. Call Read() first.");
    }

    // TODO this is just a PoC and will be changed significantly
    private static long ExtractInt64(IArrowArray column, int index)
    {
        return column switch
        {
            Int64Array arr => arr.GetValue(index) ?? 0L,
            Int32Array arr => arr.GetValue(index) ?? 0,
            Int16Array arr => arr.GetValue(index) ?? 0,
            Int8Array arr => arr.GetValue(index) ?? 0,
            Decimal128Array arr => (long)(arr.GetValue(index) ?? 0m),
            _ => throw new InvalidCastException($"Cannot convert {column.GetType().Name} to Int64."),
        };
    }

    // TODO this is just a PoC and will be changed significantly
    private static decimal ExtractDecimal(IArrowArray column, int index)
    {
        return column switch
        {
            Decimal128Array arr => arr.GetValue(index) ?? 0m,
            Int64Array arr => arr.GetValue(index) ?? 0L,
            Int32Array arr => arr.GetValue(index) ?? 0,
            Int16Array arr => arr.GetValue(index) ?? 0,
            Int8Array arr => arr.GetValue(index) ?? 0,
            _ => throw new InvalidCastException($"Cannot convert {column.GetType().Name} to Decimal."),
        };
    }

    // TODO: this implementation is just PoC to set foot in the door. It will undergo heavy refactoring.
    private object ExtractValue(IArrowArray column, int index)
    {
        if (_descriptor.Columns[0].Type == "timestamp_ltz")
        {
            var d = (Int64Array)((StructArray)column).Fields[0];
            var seconds = d.GetValue(index) ?? 0L;

            var sinceUnix = TimeSpan.FromSeconds(seconds);
            var dt = new DateTime(1970, 1, 1, 0, 0, 0, DateTimeKind.Utc) + sinceUnix;
            return new DateTimeOffset(dt, TimeSpan.Zero);
        }

        if (column is StructArray strArr)
        {
            var d = (Int64Array)strArr.Fields[0];
            var nanoseconds = d.GetValue(index) ?? 0L;

            var sinceUnix = TimeSpan.FromMilliseconds(nanoseconds / 1_000_000);
            var dt = new DateTime(1970, 1, 1) + sinceUnix;
            return new DateTimeOffset(dt, TimeSpan.Zero);
        }

        return column switch
        {
            Int64Array arr => arr.GetValue(index) ?? (object)DBNull.Value,
            Int32Array arr => (long)(arr.GetValue(index) ?? 0),
            Int16Array arr => (long)(arr.GetValue(index) ?? 0),
            Int8Array arr => (long)(arr.GetValue(index) ?? 0),
            Decimal128Array arr => arr.GetValue(index) ?? (object)DBNull.Value,
            DoubleArray arr => arr.GetValue(index) ?? (object)DBNull.Value,
            FloatArray arr => arr.GetValue(index) ?? (object)DBNull.Value,
            StringArray arr => (object?)arr.GetString(index) ?? DBNull.Value,
            BooleanArray arr => arr.GetValue(index) ?? (object)DBNull.Value,
            BinaryArray arr => (object?)arr.GetBytes(index).ToArray() ?? DBNull.Value,
            Date32Array arr => (object?)arr.GetDateTime(index) ?? DBNull.Value,
            _ => throw new NotSupportedException($"Unsupported Arrow array type: {column.GetType().Name}"),
        };
    }
}
