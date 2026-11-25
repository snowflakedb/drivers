namespace Snowflake.Data.Tests.E2E.Config;

public static class GlobalState
{
    internal static bool UseOldDriver { get; private set; }

    public sealed class DatabaseFixture : IDisposable
    {
        private readonly TimeSpan _originalOldDriverRetryTimeout;

        public DatabaseFixture()
        {
            UseOldDriver = Environment.GetEnvironmentVariable("SNOWFLAKE_DOTNET_USE_OLD_DRIVER") == "1";
            TestDriver.OverriddenFactory = new Snowflake.Data.Tests.E2E.TestConnectionFactory();
            if (UseOldDriver)
                _originalOldDriverRetryTimeout = PoolConnectionString.LowerRetryTimeoutFloorForOldDriver();
        }

        public void Dispose()
        {
            if (UseOldDriver)
                PoolConnectionString.RestoreRetryTimeoutFloorForOldDriver(_originalOldDriverRetryTimeout);
        }
    }
}
