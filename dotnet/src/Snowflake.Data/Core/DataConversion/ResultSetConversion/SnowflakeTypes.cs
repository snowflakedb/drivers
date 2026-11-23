using Snowflake.Data.Compatibility;

namespace Snowflake.Data.Core.DataConversion.ResultSetConversion;

/// <summary>
/// The single place that interprets the Snowflake type names sf_core reports in
/// <see cref="Proto.ColumnMetadata.Type"/> and says which CLR type each one is read as.
/// </summary>
internal static class SnowflakeTypes
{
    private static IReadOnlyDictionary<string, SFDataType> NameTypeMap { get; } = InitSFTypes();

    /// <summary>
    /// Looks <paramref name="name"/> up as the exact name of a type, ignoring case. Numbers, lists and
    /// <see cref="SFDataType.None"/> are not type names.
    /// </summary>
    internal static bool TryParse(string name, out SFDataType type) => NameTypeMap.TryGetValue(name, out type);

    /// <summary>
    /// The CTS type a value of <paramref name="type"/> is read as, which is also the <c>FieldType</c> of the
    /// converter that reads it. <paramref name="scale"/> matters for <see cref="SFDataType.FIXED"/> only.
    /// </summary>
    internal static Type CtsType(SFDataType type, long scale) => type switch
    {
        SFDataType.FIXED => scale == 0 ? typeof(long) : typeof(decimal),
        SFDataType.REAL => typeof(double),
        SFDataType.TEXT or SFDataType.VARIANT or SFDataType.OBJECT or SFDataType.ARRAY or SFDataType.VECTOR
            or SFDataType.MAP or SFDataType.DECFLOAT => typeof(string),
        SFDataType.DATE or SFDataType.TIME or SFDataType.TIMESTAMP_NTZ => typeof(DateTime),
        SFDataType.TIMESTAMP_LTZ or SFDataType.TIMESTAMP_TZ => typeof(DateTimeOffset),
        SFDataType.BINARY => typeof(byte[]),
        SFDataType.BOOLEAN => typeof(bool),
        _ => throw new ArgumentOutOfRangeException(nameof(type), type, "Not a column type."),
    };

    private static IReadOnlyDictionary<string, SFDataType> InitSFTypes() =>
        EnumHelper
            .GetValues<SFDataType>()
            .Where(type => type != SFDataType.None)
            .ToDictionary(type => type.ToString(), type => type, StringComparer.OrdinalIgnoreCase);
}
