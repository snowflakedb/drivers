using NSubstitute;
using Snowflake.Data.Core;
using Snowflake.Data.Core.Errors;
using Snowflake.Data.Core.Extensions;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Tests.Core;

[Trait("Category", "Unit")]
public sealed class SnowflakeExceptionTest
{
    private const string QueryId = "01bf4a3e-0001-2c4f-0000-00a1b2c3d4e5";

    private const string PublicExceptionSkipReason =
        "Does not work yet: every SnowflakeDbException constructor throws NotImplementedException (TODO SNOW-4382591 next milestone), so ToPublicException cannot build its result.";

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ServerErrorWithAVendorCodeBecomesAServerException()
    {
        var core = new ServiceException(new DriverException { Message = "SQL compilation error", VendorCode = 1003, SqlState = "42000", QueryId = QueryId });

        var exception = Assert.IsType<SnowflakeServerException>(SnowflakeException.FromCore(core));

        Assert.Equal("SQL compilation error", exception.Message);
        Assert.Equal(1003, exception.VendorCode);
        Assert.Equal("42000", exception.SqlState);
        Assert.Equal(QueryId, exception.QueryId);
        Assert.Null(exception.InnerException);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ServerErrorWithoutSqlStateOrQueryIdLeavesThemUnset()
    {
        var core = new ServiceException(new DriverException { Message = "warehouse suspended", VendorCode = 606 });

        var exception = Assert.IsType<SnowflakeServerException>(SnowflakeException.FromCore(core));

        Assert.Null(exception.SqlState);
        Assert.Null(exception.QueryId);
        Assert.Equal(606, exception.VendorCode);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ExplicitVendorCodeOfZeroStillMeansAServerError()
    {
        var core = new ServiceException(new DriverException { Message = "zero", VendorCode = 0 });

        var exception = Assert.IsType<SnowflakeServerException>(SnowflakeException.FromCore(core));

        Assert.Equal(0, exception.VendorCode);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ErrorWithoutAVendorCodeBecomesAnInternalDriverError()
    {
        var core = new ServiceException(new DriverException { Message = "handle not found", SqlState = "HY000", QueryId = QueryId });

        var exception = Assert.IsType<SnowflakeDriverException>(SnowflakeException.FromCore(core));

        Assert.Equal(SFError.INTERNAL_ERROR, exception.Error);
        Assert.Equal(["handle not found"], exception.Args);
        Assert.Equal(QueryId, exception.QueryId);
        Assert.Null(exception.InnerException);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(1003)]
    [InlineData(null)]
    public void CoreErrorDetailsDoNotReachTheCarrier(int? vendorCode)
    {
        var error = UniversalCoreErrorDetails.Error();
        if (vendorCode is { } code)
            error.VendorCode = code;
        var core = new ServiceException(error);
        Assert.Contains(UniversalCoreErrorDetails.Marker, core.Message);

        var exception = SnowflakeException.FromCore(core);

        Assert.Null(exception.InnerException);
        Assert.DoesNotContain(UniversalCoreErrorDetails.Marker, exception.ToString());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ErrorWithoutAVendorCodeOrQueryIdHasNoQueryId()
    {
        var exception = SnowflakeException.FromCore(new ServiceException(new DriverException { Message = "handle not found" }));

        Assert.Null(exception.QueryId);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TransportFailureBecomesAnInternalDriverError()
    {
        var core = new TransportException("transport down");

        var exception = Assert.IsType<SnowflakeDriverException>(SnowflakeException.FromCore(core));

        Assert.Equal(SFError.INTERNAL_ERROR, exception.Error);
        Assert.Equal(["transport down"], exception.Args);
        Assert.Null(exception.QueryId);
        Assert.Same(core, exception.InnerException);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DriverExceptionIsNamedAfterItsErrorAndKeepsItsArguments()
    {
        var cause = new InvalidOperationException("cause");

        var withoutCause = new SnowflakeDriverException(SFError.QUERY_CANCELLED, QueryId, "a", 1);
        var withCause = new SnowflakeDriverException(SFError.REQUEST_TIMEOUT, null, cause, 30);

        Assert.Equal(nameof(SFError.QUERY_CANCELLED), withoutCause.Message);
        Assert.Equal(SFError.QUERY_CANCELLED, withoutCause.Error);
        Assert.Equal(["a", 1], withoutCause.Args);
        Assert.Equal(QueryId, withoutCause.QueryId);
        Assert.Null(withoutCause.InnerException);
        Assert.Equal(nameof(SFError.REQUEST_TIMEOUT), withCause.Message);
        Assert.Equal([30], withCause.Args);
        Assert.Same(cause, withCause.InnerException);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CancellationExceptionKeepsItsErrorTokenAndArguments()
    {
        using var source = new CancellationTokenSource();
        var cause = new OperationCanceledException(source.Token);

        var exception = new SnowflakeCancellationException(SFError.REQUEST_TIMEOUT, QueryId, cause, 30);

        Assert.Equal(nameof(SFError.REQUEST_TIMEOUT), exception.Message);
        Assert.Equal(SFError.REQUEST_TIMEOUT, exception.Error);
        Assert.Equal([30], exception.Args);
        Assert.Equal(QueryId, exception.QueryId);
        Assert.Same(cause, exception.InnerException);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void QueryIdCanBeAddedAfterTheExceptionWasCreated()
    {
        var exception = new SnowflakeDriverException(SFError.EXECUTE_COMMAND_ON_CLOSED_CONNECTION, null);

        exception.QueryId = QueryId;

        Assert.Equal(QueryId, exception.QueryId);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = PublicExceptionSkipReason)]
    public void DriverExceptionBecomesASnowflakeDbExceptionWithItsErrorCodeAndQueryId()
    {
        var cause = new TransportException("transport down");
        var carrier = new SnowflakeDriverException(SFError.INTERNAL_ERROR, QueryId, cause, "transport down");

        var exception = Assert.IsType<SnowflakeDbException>(carrier.ToPublicException());

        Assert.Equal(SFError.INTERNAL_ERROR.GetErrorCode(), exception.ErrorCode);
        Assert.Equal(QueryId, exception.QueryId);
        Assert.Same(cause, exception.InnerException);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = PublicExceptionSkipReason)]
    public void ServerExceptionBecomesASnowflakeDbExceptionWithItsSqlStateAndVendorCode()
    {
        var carrier = new SnowflakeServerException("42000", 1003, "SQL compilation error", QueryId);

        var exception = Assert.IsType<SnowflakeDbException>(carrier.ToPublicException());

        Assert.Equal("42000", exception.SqlState);
        Assert.Equal(1003, exception.ErrorCode);
        Assert.Equal(QueryId, exception.QueryId);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = PublicExceptionSkipReason)]
    public void CancellationExceptionBecomesAnOperationCanceledExceptionWrappingASnowflakeDbException()
    {
        using var source = new CancellationTokenSource();
        var carrier = new SnowflakeCancellationException(SFError.QUERY_CANCELLED, QueryId, null);

        var exception = Assert.IsType<OperationCanceledException>(carrier.ToPublicException());

        Assert.Equal(source.Token, exception.CancellationToken);
        var inner = Assert.IsType<SnowflakeDbException>(exception.InnerException);
        Assert.Equal(SFError.QUERY_CANCELLED.GetErrorCode(), inner.ErrorCode);
        Assert.Equal(QueryId, inner.QueryId);
    }

    [Trait("Category", "Unit")]
    [Collection(TimeSensitiveCollection.Name)]
    public sealed class QueryStatusPollerTimingTest
    {
        private static readonly ConnectionHandle Connection = new() { Id = 3, Magic = 17 };
        private static readonly QueryId SampleQueryId = Snowflake.Data.Core.QueryId.Parse("01bf4a3e-0001-2c4f-0000-00a1b2c3d4e5");

        private readonly IDatabaseDriverService _driver = Substitute.For<IDatabaseDriverService>();

        [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
        public async Task WaitSleepsForTheScheduledDelayBetweenPollsAsync()
        {
            var delay = TimeSpan.FromMilliseconds(300);
            _driver.ConnectionGetQueryStatusAsync(Arg.Any<ConnectionGetQueryStatusRequest>(), Arg.Any<CancellationToken>())
                .Returns(new ConnectionGetQueryStatusResponse { StatusName = "RUNNING" }, new ConnectionGetQueryStatusResponse { StatusName = "SUCCESS" });
            var poller = new QueryStatusPoller(_driver, new QueryResultsRetryConfig(2, [delay, delay]));
            var stopwatch = System.Diagnostics.Stopwatch.StartNew();

            await poller.WaitUntilFinishedAsync(() => Connection, SampleQueryId, CancellationToken.None).ConfigureAwait(false);

            Assert.True(stopwatch.Elapsed >= delay - TimeSpan.FromMilliseconds(20), $"waited {stopwatch.Elapsed}, expected at least {delay}");
            _ = _driver.ReceivedWithAnyArgs(2).ConnectionGetQueryStatusAsync(default!, default);
        }
    }
}
