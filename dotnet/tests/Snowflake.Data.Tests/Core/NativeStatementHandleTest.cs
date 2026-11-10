using Microsoft.Extensions.Logging;
using NSubstitute;
using NSubstitute.ExceptionExtensions;
using Snowflake.Data.Core.Handles;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Tests.Core;

[Trait("Category", "Unit")]
public sealed class NativeStatementHandleTest
{
    private static readonly StatementHandle Handle = new() { Id = 7, Magic = 42 };

    private readonly IDatabaseDriverService _driver = Substitute.For<IDatabaseDriverService>();
    private readonly ILogger _logger = Substitute.For<ILogger>();
    private readonly NativeStatementHandle _statement;

    public NativeStatementHandleTest()
    {
        _statement = new NativeStatementHandle(_driver, Handle, _logger);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DisposeReleasesTheHandleOnce()
    {
        _statement.Dispose();

        _driver.Received(1).StatementRelease(Arg.Is<StatementReleaseRequest>(request => request.StmtHandle.Equals(Handle)));
        Assert.Single(_driver.ReceivedCalls());
        Assert.Empty(_logger.ReceivedCalls());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void SecondDisposeSendsNothing()
    {
        _statement.Dispose();
        _statement.Dispose();

        _driver.ReceivedWithAnyArgs(1).StatementRelease(default!);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void RejectedReleaseIsLoggedAndSwallowed()
    {
        _driver.StatementRelease(Arg.Any<StatementReleaseRequest>()).Throws(new ServiceException(new DriverException { Message = "release rejected" }));

        _statement.Dispose();

        _logger.Received(1).Log(LogLevel.Critical, Arg.Any<EventId>(), Arg.Any<Arg.AnyType>(), Arg.Any<ServiceException>(), Arg.Any<Func<Arg.AnyType, Exception?, string>>());
        Assert.Equal("Failed to release the statement handle", Assert.Single(_logger.ReceivedCalls()).GetArguments()[2]?.ToString());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TransportFailureDuringReleaseIsLoggedAndSwallowed()
    {
        _driver.StatementRelease(Arg.Any<StatementReleaseRequest>()).Throws(new TransportException("transport down"));

        _statement.Dispose();

        _logger.Received(1).Log(LogLevel.Critical, Arg.Any<EventId>(), Arg.Any<Arg.AnyType>(), Arg.Any<TransportException>(), Arg.Any<Func<Arg.AnyType, Exception?, string>>());
    }
}
