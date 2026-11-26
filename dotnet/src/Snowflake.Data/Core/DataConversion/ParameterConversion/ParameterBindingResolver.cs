using System.Data;
using System.Data.Common;
using Snowflake.Data.Client;

namespace Snowflake.Data.Core.DataConversion.ParameterConversion;

/// <summary>
/// Decides the Snowflake type and text a bound parameter is sent as.
/// </summary>
/// <remarks>
/// A <see cref="SnowflakeDbParameter.SFDataType"/> other than <see cref="SFDataType.None"/> takes precedence over
/// <see cref="DbParameter.DbType"/>, which <see cref="SnowflakeDbParameter"/> infers with <see cref="InferDbType"/>
/// until it is set. A value that is an array other than <c>byte[]</c> or <c>char[]</c> is an array binding with one
/// row per element; <c>byte[]</c> is a BINARY value and <c>char[]</c> a text value.
/// </remarks>
internal static class ParameterBindingResolver
{
    private static readonly Dictionary<Type, DbType> ClrTypeDbTypes = new()
    {
        [typeof(byte)] = DbType.Byte,
        [typeof(sbyte)] = DbType.SByte,
        [typeof(short)] = DbType.Int16,
        [typeof(ushort)] = DbType.UInt16,
        [typeof(int)] = DbType.Int32,
        [typeof(uint)] = DbType.UInt32,
        [typeof(long)] = DbType.Int64,
        [typeof(ulong)] = DbType.UInt64,
        [typeof(float)] = DbType.Single,
        [typeof(double)] = DbType.Double,
        [typeof(decimal)] = DbType.Decimal,
        [typeof(bool)] = DbType.Boolean,
        [typeof(string)] = DbType.String,
        [typeof(char)] = DbType.StringFixedLength,
        [typeof(char[])] = DbType.StringFixedLength,
        [typeof(Guid)] = DbType.Guid,
        [typeof(DateTime)] = DbType.DateTime,
        [typeof(DateTimeOffset)] = DbType.DateTimeOffset,
        [typeof(TimeSpan)] = DbType.Time,
        [typeof(byte[])] = DbType.Binary,
    };

    /// <summary>The CLR types <see cref="InferDbType"/> maps to a <see cref="DbType"/> other than <see cref="DbType.Object"/>.</summary>
    internal static IEnumerable<Type> InferredClrTypes => ClrTypeDbTypes.Keys;

    /// <summary>
    /// The <see cref="DbType"/> of the CLR type of <paramref name="value"/>, or of its elements for an array binding;
    /// <see cref="DbType.Object"/> for a CLR type with no counterpart.
    /// </summary>
    internal static DbType InferDbType(object value)
    {
        var type = IsArrayBinding(value) ? value.GetType().GetElementType()! : value.GetType();
        return ClrTypeDbTypes.TryGetValue(type, out var dbType) ? dbType : DbType.Object;
    }

    /// <exception cref="Errors.SnowflakeDriverException">The value of <paramref name="parameter"/> cannot be bound as its type.</exception>
    /// <exception cref="NotImplementedException">The value of <paramref name="parameter"/> is an array binding.</exception>
    internal static (SFDataType Type, string? Text) Resolve(DbParameter parameter)
    {
        var value = parameter.Value;
        if (IsArrayBinding(value))
            throw new NotImplementedException("Array binding: future milestone (SNOW-4382600), not yet implemented");

        if (parameter is SnowflakeDbParameter { SFDataType: not SFDataType.None } snowflakeParameter)
            return (snowflakeParameter.SFDataType, ParameterValueConverter.ToSnowflakeText(snowflakeParameter.SFDataType, value));

        return ParameterValueConverter.Convert(parameter.DbType, value);
    }

    private static bool IsArrayBinding(object? value) => value is Array and not (byte[] or char[]);
}
