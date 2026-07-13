using Xunit;

[assembly: AssemblyFixture(typeof(Snowflake.Data.Tests.Fixture))]

namespace Snowflake.Data.Tests;

public class Fixture
{
    public Fixture()
    {
        var currentDirectory = Directory.GetCurrentDirectory();
        var configurationMode =
#if DEBUG
            "debug";
#else
            "release";
#endif

        // TODO this path is fragile - use sth like CARGO_TARGET_DIR
        var fullPath = Path.Combine(currentDirectory, $"../../../../../../target/{configurationMode}");
        var currentDirectoryPath = Path.GetFullPath(fullPath);
        Environment.SetEnvironmentVariable("SF_CORE_LIB_PATH", currentDirectoryPath);
    }
}
