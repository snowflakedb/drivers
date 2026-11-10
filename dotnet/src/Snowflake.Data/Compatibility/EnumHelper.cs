namespace Snowflake.Data.Compatibility;

internal static class EnumHelper
{
    internal static TEnum[] GetValues<TEnum>() where TEnum : struct, Enum
    {
#if NETFRAMEWORK
        return Enum.GetValues(typeof(TEnum)).Cast<TEnum>().ToArray();
#else
        return Enum.GetValues<TEnum>();
#endif
    }
}
