using Snowflake.Data.Core.DataConversion.ResultSetConversion;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Interfaces;

/// <summary>
/// Chooses the converter that reads one result column.
/// </summary>
internal interface IColumnConverterFactory
{
    /// <summary>
    /// A new, unbound converter for <paramref name="column"/>, chosen from the column's Snowflake type in the
    /// result descriptor rather than from the Arrow data. Never throws: a type the driver does not read yet
    /// gets a converter whose reads throw <see cref="NotImplementedException"/>.
    /// </summary>
    // TODO future milestone: accept the session settings (timezone, output formats) the temporal and binary converters need.
    ColumnConverter Create(ColumnMetadata column);
}
