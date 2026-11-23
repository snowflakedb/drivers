namespace Snowflake.Data.Tests.E2E.Config;

public static class GlobalState
{
    internal static bool UseOldDriver { get; private set; }

    public sealed class DatabaseFixture : IDisposable
    {
        public DatabaseFixture()
        {
            UseOldDriver = Environment.GetEnvironmentVariable("SNOWFLAKE_DOTNET_USE_OLD_DRIVER") == "1";
            TestDriver.OverriddenFactory = new Snowflake.Data.Tests.E2E.TestConnectionFactory();
        }

        public void Dispose()
        {
        }
    }
}
