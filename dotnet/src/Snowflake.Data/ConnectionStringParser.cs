using System.Data.Common;
using Google.Protobuf.Collections;
using Snowflake.Data.Proto;

namespace Snowflake.Data;

internal sealed class ConnectionStringParser
{
    private readonly DbConnectionStringBuilder _builder = new();

    public ConnectionStringParser(string connectionString)
    {
        _builder.ConnectionString = connectionString;
    }

    /// <summary>
    /// True when the connection string had zero user-supplied parameters.
    /// </summary>
    public bool IsEmpty => _builder.Count == 0;

    /// <summary>
    /// Convert parsed key-value pairs to proto ConfigSetting map.
    /// All values are passed as strings — sf_core handles type coercion.
    /// </summary>
    public MapField<string, ConfigSetting> ToProtoOptions()
    {
        var options = new MapField<string, ConfigSetting>();
        foreach (string key in _builder.Keys)
        {
            var value = _builder[key]?.ToString();
            if (value is not null)
            {
                options[key] = new ConfigSetting { StringValue = value };
            }
        }

        return options;
    }

    /// <summary>
    /// Reads <paramref name="key"/>. The lookup is case-insensitive.
    /// </summary>
    /// <returns><see langword="true"/> when the key is present, including when its value is empty.</returns>
    internal bool TryGetValue(string key, out string value)
    {
        if (!_builder.TryGetValue(key, out var raw))
        {
            value = string.Empty;
            return false;
        }

        value = raw?.ToString() ?? string.Empty;
        return true;
    }
}
