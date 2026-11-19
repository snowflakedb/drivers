using Apache.Arrow;
using Snowflake.Data.Core.Errors;
using Snowflake.Data.Interfaces;

namespace Snowflake.Data.Core.DataConversion.ResultSetConversion;

/// <summary>
/// Reads the values of one result column out of the Arrow array of the current batch.
/// </summary>
/// <remarks>
/// One instance serves the whole result: <see cref="Bind"/> points it at the next batch's array. Every read
/// takes a row index into that array and assumes the caller validated it. A converter implements
/// <see cref="IValueReader{T}"/> for each CLR type its column can be read as, and <see cref="Get{T}"/> reads
/// through it; <see cref="GetBytes"/> and <see cref="GetChars"/> copy into a caller's buffer instead.
/// </remarks>
internal abstract class ColumnConverter
{
    private IArrowArray? _array;

    /// <param name="snowflakeType">The column's Snowflake type, reported in conversion errors.</param>
    private protected ColumnConverter(string snowflakeType)
    {
        SnowflakeType = snowflakeType;
    }

    /// <summary>The CLR type <see cref="GetValue"/> returns for a non-NULL value.</summary>
    internal abstract Type FieldType { get; }

    /// <summary>The column's Snowflake type in upper case, for example <c>FIXED</c>.</summary>
    internal string SnowflakeType { get; }

    /// <summary>
    /// Points the converter at <paramref name="array"/>, the column's data in a new batch. Throws an
    /// internal error when the Arrow type is not one sf_core sends for this column.
    /// </summary>
    internal void Bind(IArrowArray array)
    {
        Unbind();
        OnBind(array);
        _array = array;
    }

    /// <summary>Detaches the converter from the array of a batch that is being released.</summary>
    internal void Unbind()
    {
        _array = null;
        OnUnbind();
    }

    /// <summary>Whether the value at <paramref name="row"/> is NULL.</summary>
    internal bool IsNull(int row) => _array?.IsNull(row) ?? true;

    /// <summary>The value at <paramref name="row"/> in its natural CLR type, or <see cref="DBNull.Value"/>.</summary>
    internal virtual object GetValue(int row) => IsNull(row) ? DBNull.Value : GetNonNullValue(row);

    /// <summary>
    /// The value at <paramref name="row"/> as <typeparamref name="T"/>. Read as <see cref="object"/> it is
    /// <see cref="GetValue"/>: the natural CLR type, or <see cref="DBNull.Value"/>. Any other
    /// <typeparamref name="T"/> is read through the converter's <see cref="IValueReader{T}"/>, which, being
    /// covariant, also serves a base type or interface of a type the converter reads; a
    /// <typeparamref name="T"/> the column cannot be read as throws <see cref="ConversionNotSupported"/>.
    /// </summary>
    internal T? Get<T>(int row)
    {
        if (typeof(T) == typeof(object))
            return (T)GetValue(row);

        return this is IValueReader<T> reader ? reader.Read(row) : throw ConversionNotSupported(typeof(T));
    }

    /// <summary>
    /// Copies bytes of the value at <paramref name="row"/>, following <see cref="System.Data.Common.DbDataReader.GetBytes"/>.
    /// The caller has validated the offsets against <paramref name="buffer"/>.
    /// </summary>
    internal virtual long GetBytes(int row, long dataOffset, byte[]? buffer, int bufferOffset, int length) =>
        throw ConversionNotSupported(typeof(byte[]));

    /// <summary>
    /// Copies characters of the value at <paramref name="row"/>, following <see cref="System.Data.Common.DbDataReader.GetChars"/>.
    /// The caller has validated the offsets against <paramref name="buffer"/>.
    /// </summary>
    internal virtual long GetChars(int row, long dataOffset, char[]? buffer, int bufferOffset, int length) =>
        throw ConversionNotSupported(typeof(char[]));

    /// <summary>
    /// Checks the Arrow type of <paramref name="array"/> and captures the typed view the reads use.
    /// </summary>
    protected abstract void OnBind(IArrowArray array);

    /// <summary>Drops the converter's typed references to the array of the batch being released.</summary>
    protected virtual void OnUnbind()
    {
    }

    /// <summary>The value at <paramref name="row"/>, which is not NULL, in <see cref="FieldType"/>.</summary>
    protected abstract object GetNonNullValue(int row);

    /// <summary>
    /// The exception <see cref="Get{T}"/>, <see cref="GetBytes"/> and <see cref="GetChars"/> throw for a target type
    /// the column cannot be read as: <see cref="ConversionFailed"/> unless a converter overrides it.
    /// </summary>
    protected virtual Exception ConversionNotSupported(Type target) => ConversionFailed(target);

    /// <summary>
    /// The <see cref="SFError.INVALID_DATA_CONVERSION"/> carrier for a value that cannot be read as
    /// <paramref name="target"/>, with the failure the conversion raised, if any, as its cause.
    /// </summary>
    protected SnowflakeDriverException ConversionFailed(Type target, Exception? cause = null) =>
        new(SFError.INVALID_DATA_CONVERSION, null, cause, "[value hidden]", SnowflakeType, target.Name);

    /// <summary>Throws the conversion carrier when the value at <paramref name="row"/> is NULL.</summary>
    protected void ThrowIfNull(int row, Type target)
    {
        if (IsNull(row))
            throw new SnowflakeDriverException(SFError.INVALID_DATA_CONVERSION, null, "NULL", SnowflakeType, target.Name);
    }

    /// <summary>The internal error <see cref="OnBind"/> throws for an Arrow type sf_core does not send for the column.</summary>
    protected SnowflakeDriverException UnexpectedArrowType(IArrowArray array) =>
        new(SFError.INTERNAL_ERROR, null, $"Unexpected Arrow type {array.Data.DataType.Name} for a {SnowflakeType} column.");
}
