using Microsoft.Extensions.Logging;
using NSubstitute;
using NSubstitute.ExceptionExtensions;
using Snowflake.Data.Core;
using Snowflake.Data.Core.Extensions;
using Snowflake.Data.Core.Handles;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Tests.Core;

[Trait("Category", "Unit")]
public sealed class NativeResultSetHandleTest
{
    private static readonly ResultSetHandle Handle = new() { Id = 11, Magic = 99 };

    private readonly IDatabaseDriverService _driver = Substitute.For<IDatabaseDriverService>();
    private readonly ILogger _logger = Substitute.For<ILogger>();
    private readonly NativeResultSetHandle _resultSet;

    public NativeResultSetHandleTest()
    {
        _resultSet = new NativeResultSetHandle(_driver, Handle, _logger);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DisposeReleasesTheHandleOnce()
    {
        _resultSet.Dispose();

        _driver.Received(1).ResultSetRelease(Arg.Is<ResultSetReleaseRequest>(request => request.ResultSetHandle.Equals(Handle)));
        Assert.Single(_driver.ReceivedCalls());
        Assert.Empty(_logger.ReceivedCalls());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void SecondDisposeSendsNothing()
    {
        _resultSet.Dispose();
        _resultSet.Dispose();

        _driver.ReceivedWithAnyArgs(1).ResultSetRelease(default!);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void RejectedReleaseIsLoggedAndSwallowed()
    {
        _driver.ResultSetRelease(Arg.Any<ResultSetReleaseRequest>()).Throws(new ServiceException(new DriverException { Message = "release rejected" }));

        _resultSet.Dispose();

        _logger.Received(1).Log(LogLevel.Critical, Arg.Any<EventId>(), Arg.Any<Arg.AnyType>(), Arg.Any<ServiceException>(), Arg.Any<Func<Arg.AnyType, Exception?, string>>());
        Assert.Equal("Failed to release the result set handle", Assert.Single(_logger.ReceivedCalls()).GetArguments()[2]?.ToString());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TransportFailureDuringReleaseIsLoggedAndSwallowed()
    {
        _driver.ResultSetRelease(Arg.Any<ResultSetReleaseRequest>()).Throws(new TransportException("transport down"));

        _resultSet.Dispose();

        _logger.Received(1).Log(LogLevel.Critical, Arg.Any<EventId>(), Arg.Any<Arg.AnyType>(), Arg.Any<TransportException>(), Arg.Any<Func<Arg.AnyType, Exception?, string>>());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void GetStreamRequestTargetsTheHandle()
    {
        Assert.Equal(Handle, _resultSet.BuildGetStreamRequest().ResultSetHandle);
    }
}
