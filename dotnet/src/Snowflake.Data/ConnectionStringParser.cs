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
    /// True when the connection string had zero user-supplied parameters.
    /// </summary>
    public bool IsEmpty => _builder.Count == 0;
}
