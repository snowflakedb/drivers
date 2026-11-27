using System.Data.Common;
using NSubstitute;
using Snowflake.Data.Client;
using Snowflake.Data.Core;
using Snowflake.Data.Core.DataConversion.ParameterConversion;
using Snowflake.Data.Core.Extensions;

namespace Snowflake.Data.Tests;

[Trait("Category", "Unit")]
public sealed class SnowflakeDbParameterCollectionTest
{
    private readonly SnowflakeDbParameterCollection _parameters = new();
    private readonly DbParameter _otherProviderParameter = Substitute.For<DbParameter>();

    public static TheoryData<string> MembersThatTakeAParameter =>
        ["Add", "AddRange", "Insert", "Remove", "index setter", "name setter"];

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void AddWithSnowflakeTypeAppendsAParameterThatBindsAsThatType()
    {
        var parameters = new SnowflakeDbParameterCollection();

        var parameter = parameters.Add("1", SFDataType.TIMESTAMP_LTZ);
        parameter.Value = new DateTimeOffset(1970, 1, 1, 0, 0, 1, TimeSpan.Zero);

        Assert.Same(parameter, Assert.Single(parameters));
        Assert.Equal("1", parameter.ParameterName);
        Assert.Equal(SFDataType.TIMESTAMP_LTZ, parameter.SFDataType);
        Assert.Equal(
            """{"1":{"type":"TIMESTAMP_LTZ","value":"1000000000"}}""",
            System.Text.Encoding.UTF8.GetString(ParameterBindingsSerializer.Serialize(parameters)));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ParametersEnumerateAsSnowflakeDbParameters()
    {
        var first = new SnowflakeDbParameter { ParameterName = "1" };
        var second = new SnowflakeDbParameter { ParameterName = "2" };
        _parameters.Add(first);
        _parameters.Add(second);

        IEnumerable<SnowflakeDbParameter> typed = _parameters;

        Assert.Equal([first, second], typed);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ParameterOfAnotherProviderIsNeitherContainedNorFound()
    {
        _parameters.Add(new SnowflakeDbParameter { ParameterName = "1" });

        Assert.False(_parameters.Contains(_otherProviderParameter));
        Assert.Equal(-1, _parameters.IndexOf(_otherProviderParameter));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [MemberData(nameof(MembersThatTakeAParameter))]
    public void ParameterOfAnotherProviderIsRejectedAsUnsupportedBeforeTheCollectionChanges(string member)
    {
        var parameter = new SnowflakeDbParameter { ParameterName = "1" };
        _parameters.Add(parameter);

        var exception = Assert.Throws<SnowflakeDbException>(() => PassOtherProviderParameterTo(member));

        Assert.Equal(SFError.UNSUPPORTED_FEATURE.GetErrorCode(), exception.ErrorCode);
        Assert.Same(parameter, Assert.Single(_parameters));
    }

    private void PassOtherProviderParameterTo(string member)
    {
        switch (member)
        {
            case "Add":
                _parameters.Add(_otherProviderParameter);
                break;
            case "AddRange":
                _parameters.AddRange(new object[] { _otherProviderParameter });
                break;
            case "Insert":
                _parameters.Insert(0, _otherProviderParameter);
                break;
            case "Remove":
                _parameters.Remove(_otherProviderParameter);
                break;
            case "index setter":
                _parameters[0] = _otherProviderParameter;
                break;
            case "name setter":
                _parameters["1"] = _otherProviderParameter;
                break;
            default:
                throw new ArgumentOutOfRangeException(nameof(member), member, null);
        }
    }
}
