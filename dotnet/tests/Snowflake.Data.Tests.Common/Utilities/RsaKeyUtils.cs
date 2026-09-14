namespace Snowflake.Data.Tests.Utilities;

public static class RsaKeyUtils
{
    private const int MaxDirLevels = 10;
    private static ITestOutputHelper? _testOutputHelper;

    public static void Init(ITestOutputHelper? testOutputHelper) => _testOutputHelper = testOutputHelper;

    public static bool TryDiscoverRsaKeyFile(out string? keyPath)
    {
        keyPath = Environment.GetEnvironmentVariable("RSA_KEY_PATH");
        if (!string.IsNullOrEmpty(keyPath))
            return true;

        var dir = AppContext.BaseDirectory;
        var i = 0;
        for (; ; )
        {
            _testOutputHelper?.WriteLine($"Looking for {dir}..");
            var keyFiles = Directory.GetFiles(dir, "rsa_key_dotnet_*.p8");

            if (keyFiles.Length > 1)
                _testOutputHelper?.WriteLine("Multiple RSA keys found in directory. Using the first..");

            if (keyFiles.Length > 0)
            {
                var fileName = Path.GetFileName(keyFiles[0]);

                // For current directory, just return filename
                keyPath = string.Equals(dir, ".", StringComparison.Ordinal)
                    ? fileName
                    : Path.Combine(dir, fileName).Replace(Path.DirectorySeparatorChar, '/');

                return true;
            }

            dir = Directory.GetParent(dir)?.FullName ?? dir;

            if (i++ != MaxDirLevels && !string.Equals(dir, "dotnet", StringComparison.OrdinalIgnoreCase))
                continue;

            _testOutputHelper?.WriteLine("No RSA keys found.");
            return false;
        }
    }
}
