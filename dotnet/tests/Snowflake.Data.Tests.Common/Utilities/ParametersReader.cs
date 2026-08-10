using System.Text.Json;

namespace Snowflake.Data.Tests.Utilities;

public static class ParametersReader
{
    private static readonly Lazy<Dictionary<string, string>> Parameters = new(LoadParameters);
    private const int MaxDirLevels = 10;
    private static ITestOutputHelper? _testOutputHelper;

    public static void Init(ITestOutputHelper? testOutputHelper) => _testOutputHelper = testOutputHelper;

    public static string? Get(string key)
    {
        if (Parameters.Value.TryGetValue(key, out var value))
            return value;
        return Environment.GetEnvironmentVariable(key);
    }

    private static Dictionary<string, string> LoadParameters()
    {
        var parameterPath = Environment.GetEnvironmentVariable("PARAMETER_PATH");
        parameterPath = parameterPath?.Replace('/', Path.DirectorySeparatorChar);

        if (!string.IsNullOrEmpty(parameterPath) && !File.Exists(parameterPath))
        {
            _testOutputHelper?.WriteLine($"Specified path does not contain parameters.json");
            parameterPath = null;
        }

        if (string.IsNullOrEmpty(parameterPath))
        {
            // Walk up from the test assembly to find the repo root parameters.json
            var dir = AppContext.BaseDirectory;
            var i = 0;
            for (; ; )
            {
                var candidate = Path.Combine(dir, "parameters.json");
                _testOutputHelper?.WriteLine($"Looking for {candidate}..");
                if (File.Exists(candidate))
                {
                    _testOutputHelper?.WriteLine($"Found parameters at {dir}!");
                    parameterPath = candidate;
                    break;
                }

                dir = Directory.GetParent(dir)?.FullName ?? dir;

                if (i++ == MaxDirLevels)
                    throw new FileNotFoundException($"Explored {MaxDirLevels} dirs and found no parameters file!");
            }
        }

        var json = File.ReadAllText(parameterPath);
        using var doc = JsonDocument.Parse(json);

        if (!doc.RootElement.TryGetProperty("testconnection", out var testConn))
            throw new JsonException("no testconnection found!");

        var result = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        foreach (var prop in testConn.EnumerateObject())
        {
            if (prop.Value.ValueKind == JsonValueKind.String)
                result[prop.Name] = prop.Value.GetString()!;
        }
        return result;
    }
}
