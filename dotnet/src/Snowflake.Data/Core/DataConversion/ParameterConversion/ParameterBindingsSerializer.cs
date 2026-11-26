using System.Data.Common;
using System.Text.Json;

namespace Snowflake.Data.Core.DataConversion.ParameterConversion;

/// <summary>
/// Writes bound parameters as the UTF-8 JSON <c>bindings</c> object of a Snowflake query request.
/// </summary>
/// <remarks>
/// The object has one member per parameter name, <c>{"type": "&lt;SFDataType&gt;", "value": "&lt;text&gt;"}</c>, with a
/// JSON null value for a null or <see cref="DBNull"/> parameter; <see cref="ParameterBindingResolver"/> decides the
/// type and text. When two parameters share a name, the later one is bound.
/// </remarks>
internal static class ParameterBindingsSerializer
{
    /// <summary>The bindings of <paramref name="parameters"/>, or an empty array when there are none.</summary>
    /// <exception cref="Errors.SnowflakeDriverException">A parameter's value cannot be bound as its type.</exception>
    /// <exception cref="NotImplementedException">A parameter's value is an array other than <c>byte[]</c> or <c>char[]</c>.</exception>
    internal static byte[] Serialize(DbParameterCollection parameters)
    {
        if (parameters.Count == 0)
            return [];

        var bindings = new Dictionary<string, (SFDataType Type, string? Text)>();
        // TODO(SNOW-4429197): a parameter named MULTI_STATEMENT_COUNT is a statement parameter, not a binding.
        foreach (DbParameter parameter in parameters)
            bindings[parameter.ParameterName] = ParameterBindingResolver.Resolve(parameter);

        return Write(bindings);
    }

    private static byte[] Write(Dictionary<string, (SFDataType Type, string? Text)> bindings)
    {
        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream))
        {
            writer.WriteStartObject();
            foreach (var binding in bindings)
            {
                writer.WriteStartObject(binding.Key);
                writer.WriteString("type", binding.Value.Type.ToString());
                writer.WriteString("value", binding.Value.Text);
                writer.WriteEndObject();
            }

            writer.WriteEndObject();
        }

        return stream.ToArray();
    }
}
