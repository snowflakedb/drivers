using System.Text.Json;
using Xunit;

namespace Snowflake.Data.Tests.Utilities;

public static class ParametersReader
{
    private static readonly Lazy<Dictionary<string, string>> Parameters = new(LoadParameters);

    public static string? Get(string key)
    {
        if (Parameters.Value.TryGetValue(key, out var value))
            return value;
        return Environment.GetEnvironmentVariable(key);
    }

    private static Dictionary<string, string> LoadParameters()
    {
        var parameterPath = Environment.GetEnvironmentVariable("PARAMETER_PATH");
        if (string.IsNullOrEmpty(parameterPath))
        {
            // Walk up from the test assembly to find the repo root parameters.json
            var dir = AppContext.BaseDirectory;
            for (var i = 0; i < 8; i++)
            {
                var candidate = Path.Combine(dir, "parameters.json");
                TestContext.Current.TestOutputHelper?.WriteLine($"Looking for {candidate}..");
                if (File.Exists(candidate))
                {
                    TestContext.Current.TestOutputHelper?.WriteLine($"Found parameters at {dir}!");
                    parameterPath = candidate;
                    break;
                }
                dir = Path.GetDirectoryName(dir) ?? dir;
            }
            TestContext.Current.TestOutputHelper?.WriteLine($"No parameters found!");
        }

        if (string.IsNullOrEmpty(parameterPath) || !File.Exists(parameterPath))
            throw new FileLoadException("No parameters file!");

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
