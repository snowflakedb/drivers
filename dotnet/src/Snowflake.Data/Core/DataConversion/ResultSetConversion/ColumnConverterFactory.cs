using Snowflake.Data.Interfaces;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Core.DataConversion.ResultSetConversion;

/// <summary>
/// Gives every column a <see cref="NotImplementedColumnConverter"/>; the TEXT and FIXED scale 0 converters are not wired in yet.
/// </summary>
internal sealed class ColumnConverterFactory : IColumnConverterFactory
{
    /// <summary>The shared stateless instance.</summary>
    internal static IColumnConverterFactory Instance { get; } = new ColumnConverterFactory();

    public ColumnConverter Create(ColumnMetadata column)
    {
        // TODO(SNOW-4382582): choose the TEXT and FIXED scale 0 converters here; they arrive with PR stack #3380.
        return new NotImplementedColumnConverter(column.Type.ToUpperInvariant());
    }
}
