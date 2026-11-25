using System.Data.Common;
using Snowflake.Data.Tests.Assertions;
using Xunit.Sdk;

namespace Snowflake.Data.Tests;

[Trait("Category", "Unit")]
[Trait("Category", "TestingInfrastructure")]
public sealed class DbExceptionAssertTest
{
    private const string SqlState = "42000";
    private const int VendorCode = 1003;
    private const string Message = "SQL compilation error: syntax error line 1 at position 0 unexpected 'SELCT'.";

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ThrowsReturnsExceptionWhenDiagnosticsMatch()
    {
        var thrown = new DriverLikeDbException(SqlState, VendorCode, Message);

        var exception = DbExceptionAssert.Throws(
            () => throw thrown,
            sqlState: SqlState,
            errorCode: VendorCode,
            messageContains: "unexpected 'SELCT'");

        Assert.Same(thrown, exception);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ThrowsAcceptsFuncReturningValue()
    {
        var exception = DbExceptionAssert.Throws(
            () => ThrowingScalar(new DriverLikeDbException(SqlState, VendorCode, Message)),
            sqlState: SqlState,
            errorCode: VendorCode);

        Assert.Equal(VendorCode, exception.ErrorCode);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ThrowsAcceptsAnyDbExceptionWhenNoDiagnosticsAreExpected()
    {
        var thrown = new DriverLikeDbException(SqlState, VendorCode, Message);

        var exception = DbExceptionAssert.Throws(() => throw thrown);

        Assert.Same(thrown, exception);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ThrowsFailsWhenSqlStateDiffers()
    {
        Assert.Throws<EqualException>(() => DbExceptionAssert.Throws(
            () => throw new DriverLikeDbException("22000", VendorCode, Message),
            sqlState: SqlState));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ThrowsFailsWhenErrorCodeDiffers()
    {
        Assert.Throws<EqualException>(() => DbExceptionAssert.Throws(
            () => throw new DriverLikeDbException(SqlState, 100072, Message),
            errorCode: VendorCode));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ThrowsFailsWhenMessageDoesNotContainSubstring()
    {
        Assert.Throws<ContainsException>(() => DbExceptionAssert.Throws(
            () => throw new DriverLikeDbException(SqlState, VendorCode, Message),
            messageContains: "NULL result in a non-nullable column"));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ThrowsFailsWhenNonDbExceptionIsThrown()
    {
        Assert.Throws<ThrowsAnyException>(() => DbExceptionAssert.Throws(
            () => throw new NotImplementedException("TODO next milestone")));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ThrowsFailsWhenNothingIsThrown()
    {
        Assert.Throws<ThrowsAnyException>(() => DbExceptionAssert.Throws(() => { }));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ThrowsFailsWhenSqlStateIsExpectedButExceptionDeclaresNone()
    {
        Assert.Throws<EqualException>(() => DbExceptionAssert.Throws(
            () => throw new NoSqlStateDbException(VendorCode),
            sqlState: SqlState));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void HasDiagnosticsPassesWhenDiagnosticsMatch()
    {
        DbExceptionAssert.HasDiagnostics(
            new DriverLikeDbException(SqlState, VendorCode, Message),
            sqlState: SqlState,
            errorCode: VendorCode,
            messageContains: "unexpected 'SELCT'");
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void HasDiagnosticsFailsWhenErrorCodeDiffers()
    {
        Assert.Throws<EqualException>(() => DbExceptionAssert.HasDiagnostics(
            new DriverLikeDbException(SqlState, 100072, Message),
            errorCode: VendorCode));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void GetSqlStateReadsPropertyHidingTheBaseMember()
    {
        DbException exception = new DriverLikeDbException(SqlState, VendorCode, Message);

        Assert.Equal(SqlState, DbExceptionAssert.GetSqlState(exception));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void GetSqlStateReturnsNullWhenNoSqlStateIsDeclared()
    {
        Assert.Null(DbExceptionAssert.GetSqlState(new NoSqlStateDbException(VendorCode)));
    }

    private static object ThrowingScalar(DbException exception) => throw exception;

#pragma warning disable CS0108, CS0114
    private sealed class DriverLikeDbException : DbException
    {
        public DriverLikeDbException(string sqlState, int errorCode, string message) : base(message)
        {
            SqlState = sqlState;
            ErrorCode = errorCode;
        }

        public string SqlState { get; }

        public override int ErrorCode { get; }
    }
#pragma warning restore CS0108, CS0114

    private sealed class NoSqlStateDbException : DbException
    {
        public NoSqlStateDbException(int errorCode) : base("no sql state")
        {
            ErrorCode = errorCode;
        }

        public override int ErrorCode { get; }
    }
}
