using System.Data;
using System.Data.Common;
using Snowflake.Data.Core.DataConversion.ResultSetConversion;
using Snowflake.Data.Core.Errors;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Core.ResultSet;

/// <summary>
/// Describes the columns of a result as the <see cref="DataTable"/> that
/// <see cref="Client.SnowflakeDbDataReader.GetSchemaTable"/> returns.
/// </summary>
internal static class SchemaTableBuilder
{
    private const int DefaultVarcharMaxLength = 16 * 1024 * 1024;

    /// <summary>
    /// One row per column, in the columns of <see cref="SchemaTableColumn"/> that the old driver filled. The CLR
    /// type comes from the Snowflake type alone, whether the driver reads that type yet.
    /// </summary>
    /// <exception cref="SnowflakeDriverException">A column has a Snowflake type this driver does not know.</exception>
    internal static DataTable Build(ResultSetDescriptor descriptor)
    {
        var table = new DataTable("SchemaTable");
        table.Columns.Add(SchemaTableColumn.ColumnName, typeof(string));
        table.Columns.Add(SchemaTableColumn.ColumnOrdinal, typeof(int));
        table.Columns.Add(SchemaTableColumn.ColumnSize, typeof(int));
        table.Columns.Add(SchemaTableColumn.NumericPrecision, typeof(int));
        table.Columns.Add(SchemaTableColumn.NumericScale, typeof(int));
        table.Columns.Add(SchemaTableColumn.DataType, typeof(Type));
        table.Columns.Add(SchemaTableColumn.AllowDBNull, typeof(bool));
        table.Columns.Add(SchemaTableColumn.ProviderType, typeof(SFDataType));

        for (var ordinal = 0; ordinal < descriptor.Columns.Count; ordinal++)
        {
            var column = descriptor.Columns[ordinal];
            var providerType = ParseType(column.Type, descriptor.QueryId);
            var row = table.NewRow();
            row[SchemaTableColumn.ColumnName] = column.Name;
            row[SchemaTableColumn.ColumnOrdinal] = ordinal;
            row[SchemaTableColumn.ColumnSize] = ColumnSize(providerType, column);
            row[SchemaTableColumn.NumericPrecision] = (int)column.Precision;
            row[SchemaTableColumn.NumericScale] = (int)column.Scale;
            row[SchemaTableColumn.DataType] = SnowflakeTypes.CtsType(providerType, column.Scale);
            row[SchemaTableColumn.AllowDBNull] = column.Nullable;
            row[SchemaTableColumn.ProviderType] = providerType;
            table.Rows.Add(row);
        }

        return table;
    }

    private static int ColumnSize(SFDataType type, ColumnMetadata column) =>
        type is (SFDataType.ARRAY or SFDataType.OBJECT or SFDataType.VARIANT or SFDataType.MAP) && column.Length == 0
            ? DefaultVarcharMaxLength
            : (int)column.Length;

    private static SFDataType ParseType(string type, string queryId) =>
        SnowflakeTypes.TryParse(type, out var parsed) ? parsed : throw UnknownType(type, queryId);

    private static SnowflakeDriverException UnknownType(string type, string queryId) =>
        new(SFError.INTERNAL_ERROR, string.IsNullOrEmpty(queryId) ? null : queryId, $"Unknown column type: {type}");
}
