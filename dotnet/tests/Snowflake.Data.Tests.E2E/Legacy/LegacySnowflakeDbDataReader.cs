extern alias OldDriver;
using System.Collections;
using System.Data.Common;
using Snowflake.Data.Interfaces;
using OldSnowflakeDbReader = OldDriver::Snowflake.Data.Client.SnowflakeDbDataReader;

namespace Snowflake.Data.Tests.E2E.Legacy;

public sealed class LegacySnowflakeDbDataReader : ISnowflakeDbDataReader
{
    private readonly OldSnowflakeDbReader _snowflakeDbDataReaderImplementation;

    public LegacySnowflakeDbDataReader(OldSnowflakeDbReader snowflakeDbDataReaderImplementation)
    {
        _snowflakeDbDataReaderImplementation = snowflakeDbDataReaderImplementation;
    }

    public int FieldCount => _snowflakeDbDataReaderImplementation.FieldCount;

    public int RecordsAffected => _snowflakeDbDataReaderImplementation.RecordsAffected;

    public bool IsClosed => _snowflakeDbDataReaderImplementation.IsClosed;

    public bool HasRows => _snowflakeDbDataReaderImplementation.HasRows;

    public int Depth => _snowflakeDbDataReaderImplementation.Depth;

    public int VisibleFieldCount => _snowflakeDbDataReaderImplementation.VisibleFieldCount;

    public object this[int ordinal] => _snowflakeDbDataReaderImplementation[ordinal];

    public object this[string name] => _snowflakeDbDataReaderImplementation[name];

    public string GetQueryId() => _snowflakeDbDataReaderImplementation.GetQueryId();

    public bool Read() => _snowflakeDbDataReaderImplementation.Read();

    public Task<bool> ReadAsync(CancellationToken cancellationToken) => _snowflakeDbDataReaderImplementation.ReadAsync(cancellationToken);

    public Task<bool> ReadAsync() => _snowflakeDbDataReaderImplementation.ReadAsync();

    public bool NextResult() => _snowflakeDbDataReaderImplementation.NextResult();

    public bool IsDBNull(int ordinal) => _snowflakeDbDataReaderImplementation.IsDBNull(ordinal);

    public object GetValue(int ordinal) => _snowflakeDbDataReaderImplementation.GetValue(ordinal);

    public int GetValues(object[] values) => _snowflakeDbDataReaderImplementation.GetValues(values);

    public bool GetBoolean(int ordinal) => _snowflakeDbDataReaderImplementation.GetBoolean(ordinal);

    public byte GetByte(int ordinal) => _snowflakeDbDataReaderImplementation.GetByte(ordinal);

    public long GetBytes(int ordinal, long dataOffset, byte[]? buffer, int bufferOffset, int length) => _snowflakeDbDataReaderImplementation.GetBytes(ordinal, dataOffset, buffer, bufferOffset, length);

    public char GetChar(int ordinal) => _snowflakeDbDataReaderImplementation.GetChar(ordinal);

    public long GetChars(int ordinal, long dataOffset, char[]? buffer, int bufferOffset, int length) => _snowflakeDbDataReaderImplementation.GetChars(ordinal, dataOffset, buffer, bufferOffset, length);

    public DateTime GetDateTime(int ordinal) => _snowflakeDbDataReaderImplementation.GetDateTime(ordinal);

    public decimal GetDecimal(int ordinal) => _snowflakeDbDataReaderImplementation.GetDecimal(ordinal);

    public double GetDouble(int ordinal) => _snowflakeDbDataReaderImplementation.GetDouble(ordinal);

    public float GetFloat(int ordinal) => _snowflakeDbDataReaderImplementation.GetFloat(ordinal);

    public Guid GetGuid(int ordinal) => _snowflakeDbDataReaderImplementation.GetGuid(ordinal);

    public short GetInt16(int ordinal) => _snowflakeDbDataReaderImplementation.GetInt16(ordinal);

    public int GetInt32(int ordinal) => _snowflakeDbDataReaderImplementation.GetInt32(ordinal);

    public long GetInt64(int ordinal) => _snowflakeDbDataReaderImplementation.GetInt64(ordinal);

    public string GetString(int ordinal) => _snowflakeDbDataReaderImplementation.GetString(ordinal);

    public TimeSpan GetTimeSpan(int ordinal) => _snowflakeDbDataReaderImplementation.GetTimeSpan(ordinal);

    public T GetObject<T>(int ordinal) where T : class, new() => _snowflakeDbDataReaderImplementation.GetObject<T>(ordinal);

    public T[] GetArray<T>(int ordinal) => _snowflakeDbDataReaderImplementation.GetArray<T>(ordinal);

    public Dictionary<TKey, TValue> GetMap<TKey, TValue>(int ordinal) where TKey : notnull => _snowflakeDbDataReaderImplementation.GetMap<TKey, TValue>(ordinal);

    public string GetDataTypeName(int ordinal) => _snowflakeDbDataReaderImplementation.GetDataTypeName(ordinal);

    public Type GetFieldType(int ordinal) => _snowflakeDbDataReaderImplementation.GetFieldType(ordinal);

    public string GetName(int ordinal) => _snowflakeDbDataReaderImplementation.GetName(ordinal);

    public int GetOrdinal(string name) => _snowflakeDbDataReaderImplementation.GetOrdinal(name);

    public DataTable GetSchemaTable() => _snowflakeDbDataReaderImplementation.GetSchemaTable();

    public IEnumerator GetEnumerator() => _snowflakeDbDataReaderImplementation.GetEnumerator();

    public void Close() => _snowflakeDbDataReaderImplementation.Close();

    public void Dispose() => _snowflakeDbDataReaderImplementation.Dispose();

    public DbDataReader GetData(int ordinal) => _snowflakeDbDataReaderImplementation.GetData(ordinal);

    public Type GetProviderSpecificFieldType(int ordinal) => _snowflakeDbDataReaderImplementation.GetProviderSpecificFieldType(ordinal);

    public object GetProviderSpecificValue(int ordinal) => _snowflakeDbDataReaderImplementation.GetProviderSpecificValue(ordinal);

    public int GetProviderSpecificValues(object[] values) => _snowflakeDbDataReaderImplementation.GetProviderSpecificValues(values);

    public Stream GetStream(int ordinal) => _snowflakeDbDataReaderImplementation.GetStream(ordinal);

    public TextReader GetTextReader(int ordinal) => _snowflakeDbDataReaderImplementation.GetTextReader(ordinal);

    public Task<bool> IsDBNullAsync(int ordinal) => _snowflakeDbDataReaderImplementation.IsDBNullAsync(ordinal);

    public Task<bool> IsDBNullAsync(int ordinal, CancellationToken cancellationToken) => _snowflakeDbDataReaderImplementation.IsDBNullAsync(ordinal, cancellationToken);

    public Task<bool> NextResultAsync() => _snowflakeDbDataReaderImplementation.NextResultAsync();

    public Task<bool> NextResultAsync(CancellationToken cancellationToken) => _snowflakeDbDataReaderImplementation.NextResultAsync(cancellationToken);
}
