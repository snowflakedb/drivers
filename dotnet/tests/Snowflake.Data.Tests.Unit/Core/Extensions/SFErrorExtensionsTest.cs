using System.Globalization;
using Snowflake.Data.Compatibility;
using Snowflake.Data.Core;
using Snowflake.Data.Core.Extensions;

namespace Snowflake.Data.Tests.Core.Extensions;

[Trait("Category", "Unit")]
public sealed class SFErrorExtensionsTest
{
    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(SFError.INTERNAL_ERROR, 270001)]
    [InlineData(SFError.COLUMN_INDEX_OUT_OF_BOUND, 270002)]
    [InlineData(SFError.INVALID_DATA_CONVERSION, 270003)]
    [InlineData(SFError.STATEMENT_ALREADY_RUNNING_QUERY, 270004)]
    [InlineData(SFError.QUERY_CANCELLED, 270005)]
    [InlineData(SFError.DATA_READER_ALREADY_CLOSED, 270010)]
    [InlineData(SFError.UNSUPPORTED_PLATFORM, 270012)]
    [InlineData(SFError.IDP_SSO_TOKEN_URL_MISMATCH, 270040)]
    [InlineData(SFError.EXECUTE_COMMAND_ON_CLOSED_CONNECTION, 270059)]
    [InlineData(SFError.COMMAND_TEXT_NOT_SET, 270069)]
    [InlineData(SFError.INVALID_QUERY_ID_FORMAT, 270070)]
    [InlineData(SFError.UNKNOWN_QUERY_STATUS, 270071)]
    [InlineData(SFError.QUERY_RESULT_NO_DATA_RETRIES_EXCEEDED, 270072)]
    [InlineData(SFError.DATA_READER_NO_CURRENT_ROW, 270073)]
    [InlineData(SFError.DATA_READER_FETCH_IN_PROGRESS, 270074)]
    [InlineData(SFError.QUERY_RESULT_FETCH_FAILED, 270075)]
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

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void EveryDriverErrorHasAMessageTemplate()
    {
        var withoutTemplate = EnumHelper.GetValues<SFError>()
            .Where(error => error.GetErrorCode() is >= 270000 and <= 279999)
            .Where(error => error.FormatMessage("a", "b", "c") == error.ToString())
            .ToList();

        Assert.Empty(withoutTemplate);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void MessageWithoutPlaceholdersIsTheTemplate()
    {
        Assert.Equal("Query was cancelled.", SFError.QUERY_CANCELLED.FormatMessage());
        Assert.Equal("Unable to execute command due to command text not being set", SFError.COMMAND_TEXT_NOT_SET.FormatMessage());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void MessageIsTheTemplateFormattedWithTheArguments()
    {
        Assert.Equal("Request timed out after 30s.", SFError.REQUEST_TIMEOUT.FormatMessage(30));
        Assert.Equal("Failed to convert data NULL from type FIXED to type Int32.", SFError.INVALID_DATA_CONVERSION.FormatMessage("NULL", "FIXED", "Int32"));
        Assert.Equal("Failed to fetch the query result (status 22): Failed to read chunk data", SFError.QUERY_RESULT_FETCH_FAILED.FormatMessage("Failed to read chunk data", 22));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ServerErrorWithoutATemplateIsNamedAfterItself()
    {
        Assert.Equal(nameof(SFError.EXT_AUTHN_DENIED), SFError.EXT_AUTHN_DENIED.FormatMessage());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TemplateWithMorePlaceholdersThanArgumentsFailsToFormat()
    {
        Assert.Throws<FormatException>(() => SFError.INVALID_DATA_CONVERSION.FormatMessage("NULL"));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void MessageFormatsNumbersIndependentlyOfTheCurrentCulture()
    {
        var original = CultureInfo.CurrentCulture;
        try
        {
            CultureInfo.CurrentCulture = new CultureInfo("de-DE");

            Assert.Equal("Request timed out after 1.5s.", SFError.REQUEST_TIMEOUT.FormatMessage(1.5));
        }
        finally
        {
            CultureInfo.CurrentCulture = original;
        }
    }
}
