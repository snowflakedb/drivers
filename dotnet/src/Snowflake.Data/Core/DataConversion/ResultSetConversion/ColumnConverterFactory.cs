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
        if (string.Equals(column.Type, SnowflakeTypeNames.Text, StringComparison.OrdinalIgnoreCase))
            return new TextColumnConverter();

        if (string.Equals(column.Type, SnowflakeTypeNames.Fixed, StringComparison.OrdinalIgnoreCase) && column.Scale == 0)
            return new FixedColumnConverter();

        // TODO(SNOW-4483010) future milestone: converters for FIXED with scale > 0 and every other Snowflake type.
        return new NotImplementedColumnConverter(column.Type.ToUpperInvariant());
    }
}
