using Apache.Arrow;

namespace Snowflake.Data.Core.DataConversion.ResultSetConversion;

/// <summary>
/// Stands in for a column whose Snowflake type the driver does not read yet. Only <see cref="ColumnConverter.IsNull"/>
/// works; <see cref="ColumnConverter.Get{T}"/> for every type, <see cref="ColumnConverter.GetBytes"/>,
/// <see cref="ColumnConverter.GetChars"/>, <see cref="GetValue"/> and <see cref="FieldType"/> throw
/// <see cref="NotImplementedException"/>.
/// </summary>
/// <remarks>
/// This class is a temporary placeholder and will be removed once every Snowflake type has its own
/// converter. Until then the reader still opens a result that has such columns, so the columns of
/// supported types stay readable.
/// </remarks>
// TODO(SNOW-4483010) future milestone: replace with a converter per Snowflake type and delete this class.
internal sealed class NotImplementedColumnConverter : ColumnConverter
{
    private const string Message = "Future milestone, not yet implemented";

    /// <param name="snowflakeType">The column's Snowflake type in upper case.</param>
    internal NotImplementedColumnConverter(string snowflakeType)
        : base(snowflakeType)
    {
    }

    internal override Type FieldType => throw new NotImplementedException(Message);

    internal override object GetValue(int row) => throw new NotImplementedException(Message);

    protected override void OnBind(IArrowArray array)
    {
    }

    protected override object GetNonNullValue(int row) => throw new NotImplementedException(Message);

    protected override Exception ConversionNotSupported(Type target) => new NotImplementedException(Message);
}
