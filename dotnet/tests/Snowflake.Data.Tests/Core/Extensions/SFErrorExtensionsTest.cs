using Snowflake.Data.Compatibility;
using Snowflake.Data.Core.Extensions;

namespace Snowflake.Data.Tests.Core.Extensions;

[Trait("Category", "Unit")]
public sealed class SFErrorExtensionsTest
{
    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(SFError.INTERNAL_ERROR, 270001)]
    [InlineData(SFError.STATEMENT_ALREADY_RUNNING_QUERY, 270004)]
    [InlineData(SFError.QUERY_CANCELLED, 270005)]
    [InlineData(SFError.UNSUPPORTED_PLATFORM, 270012)]
    [InlineData(SFError.IDP_SSO_TOKEN_URL_MISMATCH, 270040)]
    [InlineData(SFError.EXECUTE_COMMAND_ON_CLOSED_CONNECTION, 270059)]
    [InlineData(SFError.COMMAND_TEXT_NOT_SET, 270069)]
    [InlineData(SFError.INVALID_QUERY_ID_FORMAT, 270070)]
    [InlineData(SFError.UNKNOWN_QUERY_STATUS, 270071)]
    [InlineData(SFError.QUERY_RESULT_NO_DATA_RETRIES_EXCEEDED, 270072)]
    [InlineData(SFError.SESSION_GONE, 390111)]
    [InlineData(SFError.EXT_OAUTH_ACCESS_TOKEN_EXPIRED, 390318)]
    [InlineData(SFError.EXT_OAUTH_ACCESS_TOKEN_INVALID, 390303)]
    [InlineData(SFError.ID_TOKEN_INVALID, 390195)]
    public void ErrorMapsToTheVendorCodeOfItsAttribute(SFError error, int expectedCode)
    {
        Assert.Equal(expectedCode, error.GetErrorCode());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void EveryErrorHasADistinctVendorCode()
    {
        var errors = EnumHelper.GetValues<SFError>();

        var codes = errors.Select(error => error.GetErrorCode()).ToList();

        Assert.Equal(errors.Length, codes.Distinct().Count());
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(-1)]
    [InlineData(int.MaxValue)]
    public void UndefinedErrorHasNoVendorCode(int value)
    {
        Assert.Throws<KeyNotFoundException>(() => ((SFError)value).GetErrorCode());
    }
}
