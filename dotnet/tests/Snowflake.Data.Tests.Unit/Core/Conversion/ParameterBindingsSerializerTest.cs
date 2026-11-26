using System.Data;
using System.Text.Json;
using Snowflake.Data.Client;
using Snowflake.Data.Core;
using Snowflake.Data.Core.DataConversion.ParameterConversion;
using Snowflake.Data.Core.Errors;

namespace Snowflake.Data.Tests.Core.Conversion;

[Trait("Category", "Unit")]
public sealed class ParameterBindingsSerializerTest
{
    private static readonly (object Value, string Type, string Text)[] InferredClrTypeBindingCases =
    [
        ((byte)1, "FIXED", "1"),
        ((sbyte)-1, "FIXED", "-1"),
        ((short)1, "FIXED", "1"),
        ((ushort)1, "FIXED", "1"),
        (1, "FIXED", "1"),
        (1u, "FIXED", "1"),
        (1L, "FIXED", "1"),
        (1ul, "FIXED", "1"),
        (1.5m, "FIXED", "1.5"),
        (0.1f, "REAL", "0.1"),
        (0.1d, "REAL", "0.1"),
        (true, "BOOLEAN", "True"),
        ("text", "TEXT", "text"),
        ('c', "TEXT", "c"),
        (new[] { 'a', 'b' }, "TEXT", "ab"),
        (Guid.Empty, "TEXT", "00000000-0000-0000-0000-000000000000"),
        (new DateTime(1970, 1, 1, 0, 0, 1), "TIMESTAMP_NTZ", "1000000000"),
        (new DateTimeOffset(1970, 1, 1, 0, 0, 1, TimeSpan.Zero), "TIMESTAMP_TZ", "1000000000 1440"),
        (TimeSpan.FromSeconds(1), "TIME", "1000000000"),
        (new byte[] { 0xCA }, "BINARY", "ca"),
    ];

    private readonly SnowflakeDbParameterCollection _parameters = new();

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void NoParametersMeansNoBindings()
    {
        Assert.Empty(ParameterBindingsSerializer.Serialize(_parameters));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void EachParameterIsAMemberKeyedByItsNameWithTypeAndValue()
    {
        Add("1", DbType.Int32, 42);
        Add("2", DbType.String, "zażółć \"quoted\"");
        Add("p3", DbType.Boolean, true);

        using var bindings = Serialize();

        var root = bindings.RootElement;
        Assert.Equal(3, root.EnumerateObject().Count());
        AssertBinding(root, "1", "FIXED", "42");
        AssertBinding(root, "2", "TEXT", "zażółć \"quoted\"");
        AssertBinding(root, "p3", "BOOLEAN", "True");
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void NullAndDbNullValuesAreJsonNulls()
    {
        Add("1", DbType.Int64, null);
        Add("2", DbType.Date, DBNull.Value);

        using var bindings = Serialize();

        AssertBinding(bindings.RootElement, "1", "FIXED", null);
        AssertBinding(bindings.RootElement, "2", "DATE", null);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void SFDataTypeTakesPrecedenceOverDbType()
    {
        var value = new DateTimeOffset(2024, 7, 15, 13, 45, 30, 500, TimeSpan.Zero);
        _parameters.Add(new SnowflakeDbParameter("1", SFDataType.TIMESTAMP_LTZ) { DbType = DbType.Object, Value = value });

        using var bindings = Serialize();

        AssertBinding(bindings.RootElement, "1", "TIMESTAMP_LTZ", "1721051130500000000");
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DbTypeIsInferredFromTheValueWhenNotSet()
    {
        _parameters.Add(new SnowflakeDbParameter { ParameterName = "1", Value = 7 });

        using var bindings = Serialize();

        AssertBinding(bindings.RootElement, "1", "FIXED", "7");
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void LaterParameterWithTheSameNameReplacesTheEarlierOne()
    {
        Add("1", DbType.Int32, 1);
        Add("1", DbType.String, "second");

        using var bindings = Serialize();

        Assert.Single(bindings.RootElement.EnumerateObject());
        AssertBinding(bindings.RootElement, "1", "TEXT", "second");
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ByteArrayIsAScalarBinaryValue()
    {
        Add("1", DbType.Binary, new byte[] { 0xCA, 0xFE });

        using var bindings = Serialize();

        AssertBinding(bindings.RootElement, "1", "BINARY", "cafe");
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CharArrayIsAScalarTextValueOfItsCharacters()
    {
        _parameters.Add(new SnowflakeDbParameter { ParameterName = "1", Value = new[] { 'a', 'b' } });

        using var bindings = Serialize();

        AssertBinding(bindings.RootElement, "1", "TEXT", "ab");
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [MemberData(nameof(InferredClrTypeBindings))]
    public void ValueOfEveryInferredClrTypeBindsWithoutAnExplicitType(object value, string type, string text)
    {
        _parameters.Add(new SnowflakeDbParameter { ParameterName = "1", Value = value });

        using var bindings = Serialize();

        AssertBinding(bindings.RootElement, "1", type, text);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void InferredClrTypeBindingsCoverEveryInferredClrType()
    {
        var covered = InferredClrTypeBindingCases.Select(binding => binding.Value.GetType());

        Assert.Equal(ParameterBindingResolver.InferredClrTypes.OrderBy(t => t.FullName), covered.OrderBy(t => t.FullName));
    }

    public static TheoryData<object, string, string> InferredClrTypeBindings()
    {
        var data = new TheoryData<object, string, string>();
        foreach (var (value, type, text) in InferredClrTypeBindingCases)
            data.Add(value, type, text);

        return data;
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ArrayValueIsNotSupportedYet()
    {
        Add("1", DbType.Int32, new[] { 1, 2, 3 });

        var exception = Assert.Throws<NotImplementedException>(() => ParameterBindingsSerializer.Serialize(_parameters));

        Assert.Contains("SNOW-4382600", exception.Message);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ConversionFailureOfAnyParameterFailsTheSerialization()
    {
        Add("1", DbType.Int32, 1);
        Add("2", DbType.Currency, 1.0m);

        var exception = Assert.Throws<SnowflakeDriverException>(() => ParameterBindingsSerializer.Serialize(_parameters));

        Assert.Equal(SFError.UNSUPPORTED_DOTNET_TYPE, exception.Error);
    }

    private static void AssertBinding(JsonElement root, string name, string type, string? value)
    {
        var binding = root.GetProperty(name);
        Assert.Equal(2, binding.EnumerateObject().Count());
        Assert.Equal(type, binding.GetProperty("type").GetString());
        Assert.Equal(value, binding.GetProperty("value").GetString());
        if (value is null)
            Assert.Equal(JsonValueKind.Null, binding.GetProperty("value").ValueKind);
    }

    private void Add(string name, DbType dbType, object? value) =>
        _parameters.Add(new SnowflakeDbParameter { ParameterName = name, DbType = dbType, Value = value });

    private JsonDocument Serialize() => JsonDocument.Parse(ParameterBindingsSerializer.Serialize(_parameters));
}
