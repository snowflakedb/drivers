using Snowflake.Data.Interfaces;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Core.DataConversion.ResultSetConversion;

/// <summary>
/// Reads TEXT and FIXED with scale 0; every other type gets a <see cref="NotImplementedColumnConverter"/>.
/// </summary>
internal sealed class ColumnConverterFactory : IColumnConverterFactory
{
    /// <summary>The shared stateless instance.</summary>
    internal static IColumnConverterFactory Instance { get; } = new ColumnConverterFactory();

    public ColumnConverter Create(ColumnMetadata column)
    {
        if (!SnowflakeTypes.TryParse(column.Type, out var type))
            return new NotImplementedColumnConverter(column.Type.ToUpperInvariant());

        return type switch
        {
            SFDataType.TEXT => new TextColumnConverter(),
            SFDataType.FIXED when column.Scale == 0 => new FixedColumnConverter(),
            _ => new NotImplementedColumnConverter(column.Type.ToUpperInvariant())
        };
    }
}
