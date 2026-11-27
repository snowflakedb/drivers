using System.Data.Common;
using Snowflake.Data.Client;
using Snowflake.Data.Core;
using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests;

[Trait("Category", "Unit")]
public sealed class SnowflakeDbExceptionTest
{
    private const string QueryId = "01bf4a3e-0001-2c4f-0000-00a1b2c3d4e5";

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ServerFailureKeepsItsSqlStateVendorCodeAndQueryId()
    {
        var exception = new SnowflakeDbException("42000", 1003, "SQL compilation error", QueryId);

        Assert.Equal("42000", exception.SqlState);
        Assert.Equal(1003, exception.ErrorCode);
        Assert.Equal(QueryId, exception.QueryId);
        Assert.Null(exception.InnerException);
        Assert.Equal($"Error: SQL compilation error SqlState: 42000, VendorCode: 1003, QueryId: {QueryId}", exception.Message);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DriverErrorWithAQueryIdFormatsItsMessageWithTheCause()
    {
        var cause = new InvalidOperationException("cause");

        var exception = new SnowflakeDbException(SFError.INTERNAL_ERROR, QueryId, cause);

        Assert.Null(exception.SqlState);
        Assert.Equal(270001, exception.ErrorCode);
        Assert.Equal(QueryId, exception.QueryId);
        Assert.Same(cause, exception.InnerException);
        Assert.Equal($"Error: Snowflake Internal Error: cause SqlState: , VendorCode: 270001, QueryId: {QueryId}", exception.Message);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DriverErrorFormatsItsArgumentsAndHasNoSqlStateOrQueryId()
    {
        var exception = new SnowflakeDbException(SFError.COLUMN_INDEX_OUT_OF_BOUND, 7);

        Assert.Null(exception.SqlState);
        Assert.Equal(270002, exception.ErrorCode);
        Assert.Null(exception.QueryId);
        Assert.Null(exception.InnerException);
        Assert.Equal("Error: Column index 7 is out of bound of valid index. SqlState: , VendorCode: 270002, QueryId: ", exception.Message);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DriverErrorWithASqlStateKeepsIt()
    {
        var exception = new SnowflakeDbException("08006", SFError.INTERNAL_ERROR, "Unable to connect");

        Assert.Equal("08006", exception.SqlState);
        Assert.Equal(270001, exception.ErrorCode);
        Assert.Equal("Error: Snowflake Internal Error: Unable to connect SqlState: 08006, VendorCode: 270001, QueryId: ", exception.Message);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DriverErrorWithACauseKeepsIt()
    {
        var cause = new InvalidOperationException("cause");

        var exception = new SnowflakeDbException(cause, SFError.REQUEST_TIMEOUT, 30);

        Assert.Same(cause, exception.InnerException);
        Assert.Null(exception.SqlState);
        Assert.Equal(270007, exception.ErrorCode);
        Assert.Equal("Error: Request timed out after 30s. SqlState: , VendorCode: 270007, QueryId: ", exception.Message);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DriverErrorWithACauseAndASqlStateKeepsBoth()
    {
        var cause = new InvalidOperationException("cause");

        var exception = new SnowflakeDbException(cause, "0A000", SFError.INTERNAL_ERROR, "gzip");

        Assert.Same(cause, exception.InnerException);
        Assert.Equal("0A000", exception.SqlState);
        Assert.Equal(270001, exception.ErrorCode);
        Assert.Equal("Error: Snowflake Internal Error: gzip SqlState: 0A000, VendorCode: 270001, QueryId: ", exception.Message);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void QueryIdCanBeSetAfterTheExceptionWasCreated()
    {
        var exception = new SnowflakeDbException(SFError.QUERY_CANCELLED);

        exception.QueryId = QueryId;

        Assert.Equal(QueryId, exception.QueryId);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void SqlStateReadThroughDbExceptionMatchesTheSnowflakeValue()
    {
        var baseSqlState = typeof(DbException).GetProperty(nameof(SnowflakeDbException.SqlState));
        Skip.When(baseSqlState is null, "DbException has no SqlState property before .NET 5");

        DbException exception = new SnowflakeDbException("42000", 1003, "SQL compilation error", QueryId);

        Assert.Equal("42000", baseSqlState!.GetValue(exception));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ErrorCodeReadThroughDbExceptionIsTheVendorCode()
    {
        DbException exception = new SnowflakeDbException("42000", 1003, "SQL compilation error", QueryId);

        Assert.Equal(1003, exception.ErrorCode);
    }
}
