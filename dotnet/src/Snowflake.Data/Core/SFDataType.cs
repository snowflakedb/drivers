// ReSharper disable InconsistentNaming
namespace Snowflake.Data.Core;

/// <summary>
/// The Snowflake type of a column, reported as <see cref="System.Data.SchemaTableColumn.ProviderType"/> in the table
/// <see cref="Client.SnowflakeDbDataReader.GetSchemaTable"/> returns.
/// </summary>
public enum SFDataType
{
    None,
    FIXED,
    REAL,
    TEXT,
    DATE,
    VARIANT,
    TIMESTAMP_LTZ,
    TIMESTAMP_NTZ,
    TIMESTAMP_TZ,
    OBJECT,
    BINARY,
    TIME,
    BOOLEAN,
    ARRAY,
    MAP,
    VECTOR,
    DECFLOAT,
}
