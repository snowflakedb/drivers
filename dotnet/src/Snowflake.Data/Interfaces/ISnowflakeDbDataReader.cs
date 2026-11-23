using System.Collections;
using System.Data;
using System.Data.Common;

namespace Snowflake.Data.Interfaces;

internal interface ISnowflakeDbDataReader
{
    int FieldCount { get; }
    int RecordsAffected { get; }
    bool IsClosed { get; }

    bool HasRows { get; }

    int Depth { get; }
    int VisibleFieldCount { get; }
    object this[int ordinal] { get; }
    object this[string name] { get; }

    string GetQueryId();

    bool Read();
    Task<bool> ReadAsync(CancellationToken cancellationToken);
    Task<bool> ReadAsync();
    bool NextResult();
    bool IsDBNull(int ordinal);
    object GetValue(int ordinal);
    int GetValues(object[] values);
    bool GetBoolean(int ordinal);
    byte GetByte(int ordinal);
    long GetBytes(int ordinal, long dataOffset, byte[]? buffer, int bufferOffset, int length);
    char GetChar(int ordinal);
    long GetChars(int ordinal, long dataOffset, char[]? buffer, int bufferOffset, int length);
    DateTime GetDateTime(int ordinal);
    decimal GetDecimal(int ordinal);
    double GetDouble(int ordinal);
    float GetFloat(int ordinal);
    Guid GetGuid(int ordinal);
    short GetInt16(int ordinal);
    int GetInt32(int ordinal);
    long GetInt64(int ordinal);

    string GetString(int ordinal);

    TimeSpan GetTimeSpan(int ordinal);

    T GetObject<T>(int ordinal)
        where T : class, new();

    T[] GetArray<T>(int ordinal);

    Dictionary<TKey, TValue> GetMap<TKey, TValue>(int ordinal)
        where TKey : notnull;

    string GetDataTypeName(int ordinal);

    Type GetFieldType(int ordinal);
    string GetName(int ordinal);

    int GetOrdinal(string name);

    DataTable GetSchemaTable();

    IEnumerator GetEnumerator();
    void Close();
    void Dispose();
    DbDataReader GetData(int ordinal);
    Type GetProviderSpecificFieldType(int ordinal);
    object GetProviderSpecificValue(int ordinal);
    int GetProviderSpecificValues(object[] values);
    Stream GetStream(int ordinal);
    TextReader GetTextReader(int ordinal);
    Task<bool> IsDBNullAsync(int ordinal);
    Task<bool> IsDBNullAsync(int ordinal, CancellationToken cancellationToken);
    Task<bool> NextResultAsync();
    Task<bool> NextResultAsync(CancellationToken cancellationToken);
}
