namespace Snowflake.Data.Compatibility;

internal static class ArgumentNullExceptionHelper
{
    public static void ThrowIfNull(object? value, string paramName)
    {
#if NETFRAMEWORK
        if (value == null)
            throw new ArgumentNullException(paramName);
#else
        ArgumentNullException.ThrowIfNull(value, paramName);
#endif
    }
}
