using System.Data;
using Snowflake.Data.Client;
using Snowflake.Data.Core;

namespace Snowflake.Data.Tests;

[Trait("Category", "Unit")]
public sealed class SnowflakeDbParameterTest
{
    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void NullParameterNameReadsAsEmpty()
    {
        var parameter = new SnowflakeDbParameter { ParameterName = "1" };

        parameter.ParameterName = null;

        Assert.Equal(string.Empty, parameter.ParameterName);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [MemberData(nameof(ValuesAndInferredDbTypes))]
    public void DbTypeIsInferredFromTheValueWhenNotSet(object value, DbType expected)
    {
        var parameter = new SnowflakeDbParameter { Value = value };

        Assert.Equal(expected, parameter.DbType);
    }

    public static TheoryData<object, DbType> ValuesAndInferredDbTypes() => new()
    {
        { (byte)1, DbType.Byte },
        { (sbyte)1, DbType.SByte },
        { (short)1, DbType.Int16 },
        { (ushort)1, DbType.UInt16 },
        { 1, DbType.Int32 },
        { 1u, DbType.UInt32 },
        { 1L, DbType.Int64 },
        { 1ul, DbType.UInt64 },
        { 1f, DbType.Single },
        { 1d, DbType.Double },
        { 1m, DbType.Decimal },
        { true, DbType.Boolean },
        { "text", DbType.String },
        { 'c', DbType.StringFixedLength },
        { Guid.Empty, DbType.Guid },
        { new DateTime(2024, 1, 1), DbType.DateTime },
        { new DateTimeOffset(2024, 1, 1, 0, 0, 0, TimeSpan.Zero), DbType.DateTimeOffset },
        { TimeSpan.FromHours(1), DbType.Time },
        { new byte[] { 1 }, DbType.Binary },
        { new[] { 1, 2 }, DbType.Int32 },
        { new[] { "a", "b" }, DbType.String },
        { new[] { 'a' }, DbType.StringFixedLength },
    };

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ValueOfAnUnmappedClrTypeInfersObject()
    {
        Assert.Equal(DbType.Object, new SnowflakeDbParameter { Value = new object() }.DbType);
        Assert.Equal(DbType.Object, new SnowflakeDbParameter { Value = new List<int> { 1 } }.DbType);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(null)]
    [InlineData("dbnull")]
    public void DbTypeOfANullValueIsTheDefault(string? valueKind)
    {
        var parameter = new SnowflakeDbParameter { Value = valueKind is null ? null : DBNull.Value };

        Assert.Equal(DbType.AnsiString, parameter.DbType);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ExplicitDbTypeWinsOverTheValueType()
    {
        var parameter = new SnowflakeDbParameter { DbType = DbType.Decimal, Value = 1 };

        Assert.Equal(DbType.Decimal, parameter.DbType);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DefaultParameterBindsAsAnInputWithoutSFDataType()
    {
        var parameter = new SnowflakeDbParameter();

        Assert.Equal(SFDataType.None, parameter.SFDataType);
        Assert.Equal(ParameterDirection.Input, parameter.Direction);
        Assert.Equal(string.Empty, parameter.ParameterName);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ResetDbTypeRestoresTheSFDataTypeGivenByName()
    {
        var parameter = new SnowflakeDbParameter("p0", SFDataType.TIMESTAMP_LTZ) { DbType = DbType.Int32 };
        parameter.SFDataType = SFDataType.TEXT;

        parameter.ResetDbType();

        Assert.Equal("p0", parameter.ParameterName);
        Assert.Equal(SFDataType.TIMESTAMP_LTZ, parameter.SFDataType);
        Assert.Equal(DbType.Int32, parameter.DbType);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ResetDbTypeClearsTheSFDataTypeGivenByIndex()
    {
        var parameter = new SnowflakeDbParameter(3, SFDataType.TIMESTAMP_TZ);

        parameter.ResetDbType();

        Assert.Equal("3", parameter.ParameterName);
        Assert.Equal(SFDataType.None, parameter.SFDataType);
    }
}
