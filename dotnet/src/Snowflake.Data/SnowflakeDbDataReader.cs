using System.Collections;
using System.Data.Common;
using Apache.Arrow;
using Apache.Arrow.C;
using Apache.Arrow.Ipc;
using Snowflake.Data.Proto;

namespace Snowflake.Data;

// TODO this implementation is just PoC and will undergo heavy refactoring.
public sealed class SnowflakeDbDataReader : DbDataReader
{
    private readonly IDatabaseDriverService _driver;
    private readonly ResultSetHandle _resultSetHandle;
    private readonly ResultSetDescriptor _descriptor;
    private readonly IArrowArrayStream _arrowStream;

    private RecordBatch? _currentBatch;
    private int _rowIndexInBatch = -1;
    private bool _closed;
    private bool _exhausted;

    internal unsafe SnowflakeDbDataReader(
        IDatabaseDriverService driver,
        ResultSetHandle resultSetHandle,
        ArrowArrayStreamPtr arrowStreamPtr,
        ResultSetDescriptor descriptor)
    {
        _driver = driver;
        _resultSetHandle = resultSetHandle;
        _descriptor = descriptor;

        // Import the native Arrow C Data Interface stream pointer.
        var ptrBytes = arrowStreamPtr.Value.ToByteArray();
        var nativePtr = (nint)BitConverter.ToInt64(ptrBytes, 0);
        var cStream = (CArrowArrayStream*)nativePtr;
        _arrowStream = CArrowArrayStreamImporter.ImportArrayStream(cStream);
    }

    public override int FieldCount => _descriptor.Columns.Count;

    public override int RecordsAffected => _descriptor.HasRowsAffected ? (int)_descriptor.RowsAffected : -1;

    public override bool HasRows => !_descriptor.HasRowsAffected || _descriptor.RowsAffected > 0;

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
        return ExtractDecimal(column, _rowIndexInBatch);
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

    public override DateTime GetDateTime(int ordinal) => throw new NotImplementedException();

    public override double GetDouble(int ordinal) => throw new NotImplementedException();

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

        try
        {
            _driver.ResultSetRelease(new ResultSetReleaseRequest
            {
                ResultSetHandle = _resultSetHandle,
            });
        }
        catch
        {
            // Best-effort release; swallow exceptions during cleanup.
        }
    }

    // TODO: this implementation is just PoC and will undergo heavy refactoring.
    protected override void Dispose(bool disposing)
    {
        if (disposing)
            Close();

        base.Dispose(disposing);
    }

    private void EnsurePositioned()
    {
        if (_currentBatch is null || _rowIndexInBatch < 0)
            throw new InvalidOperationException("No current row. Call Read() first.");
    }

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

    private static object ExtractValue(IArrowArray column, int index)
    {
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
            _ => throw new NotSupportedException($"Unsupported Arrow array type: {column.GetType().Name}"),
        };
    }
}
