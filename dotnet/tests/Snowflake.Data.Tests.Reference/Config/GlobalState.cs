using Snowflake.Data.Tests.Reference.Config;

[assembly: AssemblyFixture(typeof(GlobalState.DatabaseFixture))]
namespace Snowflake.Data.Tests.Reference.Config;

public static class GlobalState
{
    internal static bool UseOldDriver { get; private set; }

    public sealed class DatabaseFixture : IDisposable
    {
        public DatabaseFixture()
        {
            UseOldDriver = Environment.GetEnvironmentVariable("SNOWFLAKE_DOTNET_USE_OLD_DRIVER") == "1";
            ITFixture.OverriddenFactory = new TestConnectionFactory();
        }

        public void Dispose()
        { }
    }
}


