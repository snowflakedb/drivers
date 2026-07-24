namespace Snowflake.Data.Tests;

public enum ConfigurationMode
{
    Debug = 1,
    Release = 2,
}

public static class Env
{
    public static ConfigurationMode ConfigurationMode =>
#if DEBUG
            ConfigurationMode.Debug;
#else
            ConfigurationMode.Release;
#endif
}
