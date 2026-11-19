using Snowflake.Data.Proto;

namespace Snowflake.Data.Core.DataConversion.ResultSetConversion;

/// <summary>The Snowflake type names sf_core reports in <see cref="ColumnMetadata.Type"/>, compared case-insensitively.</summary>
internal static class SnowflakeTypeNames
{
    /// <summary>NUMBER and its aliases.</summary>
    internal const string Fixed = "FIXED";

    /// <summary>VARCHAR and its aliases.</summary>
    internal const string Text = "TEXT";
}
