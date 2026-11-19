using System.Data.Common;
using System.Diagnostics;
using Google.Protobuf;
using Microsoft.Extensions.Logging.Abstractions;
using NSubstitute;
using NSubstitute.ExceptionExtensions;
using NSubstitute.Extensions;
using Snowflake.Data.Core;
using Snowflake.Data.Core.Errors;
using Snowflake.Data.Core.Extensions;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Proto;
using Snowflake.Data.Telemetry;

namespace Snowflake.Data.Tests;

[Trait("Category", "Unit")]
public sealed class SnowflakeDbCommandTest : IDisposable
{
    private const string Sql = "SELECT 1";
    private const string ValidQueryId = "01bf4a3e-0001-2c4f-0000-00a1b2c3d4e5";

    private const string PublicExceptionSkipReason =
        "Does not work yet: every SnowflakeDbException constructor throws NotImplementedException (TODO next milestone), so no test can observe the SnowflakeDbException this path throws.";

    private readonly ISnowflakeStatement _statement = Substitute.For<ISnowflakeStatement>();
    private readonly SnowflakeDbCommand _command;

    public SnowflakeDbCommandTest()
    {
        _command = new SnowflakeDbCommand(_statement, NullLogger<SnowflakeDbCommand>.Instance)
        {
            Connection = OpenConnection(),
            CommandText = Sql,
        };
    }

    public void Dispose() => _command.Dispose();

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ExecuteNonQueryPassesTheCommandSettingsToTheStatement()
    {
        _statement.ExecuteNonQuery(default, default).ReturnsForAnyArgs(7);
        _command.QueryTag = "nightly-load";
        _command.CommandTimeout = 30;

        var count = _command.ExecuteNonQuery();

        Assert.Equal(7, count);
        _statement.Received(1).ExecuteNonQuery(
            new SnowflakeStatementCtx(Sql, "nightly-load", TimeSpan.FromSeconds(30), DescribeOnly: false),
            Arg.Is<Activity?>(activity => activity != null && activity.OperationName == TelemetryActivities.ExecuteNonQuery));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(0)]
    [InlineData(-1)]
    [InlineData(int.MinValue)]
    public void CommandTimeoutThatIsNotPositiveMeansNoLimit(int commandTimeout)
    {
        _command.CommandTimeout = commandTimeout;

        _command.ExecuteNonQuery();

        _statement.Received(1).ExecuteNonQuery(Arg.Is<SnowflakeStatementCtx>(ctx => ctx.CommandTimeout == null), Arg.Any<Activity?>());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void QueryTagDefaultsToNull()
    {
        _command.ExecuteNonQuery();

        _statement.Received(1).ExecuteNonQuery(Arg.Is<SnowflakeStatementCtx>(ctx => ctx.QueryTag == null), Arg.Any<Activity?>());
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(0, false)]
    [InlineData(8, false)]
    [InlineData(2, true)]
    [InlineData(6, true)]
    public void ExecuteReaderDescribesOnlyForSchemaOnlyBehavior(int behavior, bool describeOnly)
    {
        var behaviorEnum = (CommandBehavior)behavior;
        _command.ExecuteReader(behaviorEnum);

        _statement.Received(1).ExecuteReader(new SnowflakeStatementCtx(Sql, null, null, describeOnly));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task ExecuteReaderAsyncDescribesOnlyForSchemaOnlyBehaviorAsync()
    {
        await _command.ExecuteReaderAsync(CommandBehavior.SchemaOnly, CancellationToken.None).ConfigureAwait(false);

        _ = _statement.Received(1).ExecuteReaderAsync(new SnowflakeStatementCtx(Sql, null, null, DescribeOnly: true), Arg.Any<CancellationToken>());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ExecuteInAsyncModeReturnsTheSubmittedQueryId()
    {
        _statement.ExecuteInAsyncMode(default).ReturnsForAnyArgs(ValidQueryId);

        var queryId = _command.ExecuteInAsyncMode();

        Assert.Equal(ValidQueryId, queryId);
        _statement.Received(1).ExecuteInAsyncMode(new SnowflakeStatementCtx(Sql, null, null));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task ExecuteAsyncInAsyncModeReturnsTheSubmittedQueryIdAsync()
    {
        _statement.ExecuteInAsyncModeAsync(default, default).ReturnsForAnyArgs(ValidQueryId);

        var queryId = await _command.ExecuteAsyncInAsyncMode(CancellationToken.None).ConfigureAwait(false);

        Assert.Equal(ValidQueryId, queryId);
        _ = _statement.Received(1).ExecuteInAsyncModeAsync(new SnowflakeStatementCtx(Sql, null, null), Arg.Any<CancellationToken>());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task GetQueryStatusPassesTheParsedQueryIdAsync()
    {
        _statement.GetQueryStatus(default).ReturnsForAnyArgs(QueryStatus.Running);
        _statement.GetQueryStatusAsync(default, default).ReturnsForAnyArgs(QueryStatus.Success);

        Assert.Equal(QueryStatus.Running, _command.GetQueryStatus(ValidQueryId));
        Assert.Equal(QueryStatus.Success, await _command.GetQueryStatusAsync(ValidQueryId, CancellationToken.None).ConfigureAwait(false));

        _statement.Received(1).GetQueryStatus(QueryId.Parse(ValidQueryId));
        _ = _statement.Received(1).GetQueryStatusAsync(QueryId.Parse(ValidQueryId), Arg.Any<CancellationToken>());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task GetResultsFromQueryIdPassesTheParsedQueryIdAsync()
    {
        _command.GetResultsFromQueryId(ValidQueryId);
        await _command.GetResultsFromQueryIdAsync(ValidQueryId, CancellationToken.None).ConfigureAwait(false);

        _statement.Received(1).GetResultsFromQueryId(QueryId.Parse(ValidQueryId), null);
        _ = _statement.Received(1).GetResultsFromQueryIdAsync(QueryId.Parse(ValidQueryId), null, Arg.Any<CancellationToken>());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task GetResultsFromQueryIdPassesTheCommandTimeoutAsync()
    {
        _command.CommandTimeout = 45;

        _command.GetResultsFromQueryId(ValidQueryId);
        await _command.GetResultsFromQueryIdAsync(ValidQueryId, CancellationToken.None).ConfigureAwait(false);

        _statement.Received(1).GetResultsFromQueryId(QueryId.Parse(ValidQueryId), TimeSpan.FromSeconds(45));
        _ = _statement.Received(1).GetResultsFromQueryIdAsync(QueryId.Parse(ValidQueryId), TimeSpan.FromSeconds(45), Arg.Any<CancellationToken>());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ParametersAreNotSupportedYet()
    {
        _command.Parameters.Add(_command.CreateParameter());

        var exception = Assert.Throws<NotImplementedException>(() => _command.ExecuteNonQuery());

        Assert.Contains("SNOW-4382583", exception.Message);
        Assert.Empty(_statement.ReceivedCalls());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task ExceptionThatIsNotADriverFailureReachesTheCallerUnchangedAsync()
    {
        var failure = new InvalidOperationException("unexpected");
        _statement.ExecuteNonQuery(default, default).ThrowsForAnyArgs(failure);
        _statement.ExecuteNonQueryAsync(default, default, default).ReturnsForAnyArgs(Task.FromException<int>(failure));

        var syncException = Assert.Throws<InvalidOperationException>(() => _command.ExecuteNonQuery());
        var asyncException = await Assert.ThrowsAsync<InvalidOperationException>(
            () => _command.ExecuteNonQueryAsync(CancellationToken.None)).ConfigureAwait(false);

        Assert.Same(failure, syncException);
        Assert.Same(failure, asyncException);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task AlreadyCancelledTokenStopsAsyncExecutionBeforeTheStatementAsync()
    {
        using var cancellation = new CancellationTokenSource();
        cancellation.Cancel();

        await Assert.ThrowsAnyAsync<OperationCanceledException>(
            () => _command.ExecuteNonQueryAsync(cancellation.Token)).ConfigureAwait(false);
        await Assert.ThrowsAnyAsync<OperationCanceledException>(
            () => _command.ExecuteScalarAsync(cancellation.Token)).ConfigureAwait(false);

        Assert.Empty(_statement.ReceivedCalls());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CancelReachesTheStatementOnlyAfterAnExecution()
    {
        _command.Cancel();
        _statement.DidNotReceive().Cancel();

        _command.ExecuteNonQuery();
        _command.Cancel();
        _statement.Received(1).Cancel();
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void GetQueryIdReportsTheStatementsLastQueryIdAfterAnExecution()
    {
        _statement.LastQueryId.Returns(ValidQueryId);

        Assert.Null(_command.GetQueryId());

        _command.ExecuteNonQuery();
        Assert.Equal(ValidQueryId, _command.GetQueryId());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DisposeBeforeAnyExecutionDoesNotReachTheStatement()
    {
        _command.Dispose();

        _statement.DidNotReceive().Dispose();
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DisposeAfterAnExecutionDisposesTheStatement()
    {
        _command.ExecuteNonQuery();

        _command.Dispose();

        _statement.Received(1).Dispose();
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ConsecutiveExecutionsReuseTheStatement()
    {
        _command.ExecuteNonQuery();
        _command.CommandText = "SELECT 2";
        _command.ExecuteInAsyncMode();

        Received.InOrder(() =>
        {
            _statement.ExecuteNonQuery(new SnowflakeStatementCtx("SELECT 1", null, null), Arg.Any<Activity?>());
            _statement.ExecuteInAsyncMode(new SnowflakeStatementCtx("SELECT 2", null, null));
        });
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ConstructorWithAConnectionUsesIt()
    {
        var connection = new SnowflakeDbConnection(Substitute.For<ICoreTransport>());

        using var command = new SnowflakeDbCommand(connection);

        Assert.Same(connection, command.Connection);
        Assert.Equal(string.Empty, command.CommandText);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ConstructorWithAConnectionAndCommandTextUsesBoth()
    {
        var connection = new SnowflakeDbConnection(Substitute.For<ICoreTransport>());

        using var command = new SnowflakeDbCommand(connection, "SELECT 2");

        Assert.Same(connection, command.Connection);
        Assert.Equal("SELECT 2", command.CommandText);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CommandTypeIsTextAndAcceptsText()
    {
        _command.CommandType = CommandType.Text;

        Assert.Equal(CommandType.Text, _command.CommandType);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DesignTimeVisibleIsOffAndCanBeSetOff()
    {
        _command.DesignTimeVisible = false;

        Assert.False(_command.DesignTimeVisible);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void UpdatedRowSourceIsNoneAndAcceptsNone()
    {
        _command.UpdatedRowSource = UpdateRowSource.None;

        Assert.Equal(UpdateRowSource.None, _command.UpdatedRowSource);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = PublicExceptionSkipReason)]
    public async Task AsyncDriverFailureFromTheStatementReachesTheCallerAsSnowflakeDbExceptionAsync()
    {
        _statement.ExecuteNonQueryAsync(default, default, default).ReturnsForAnyArgs(
            Task.FromException<int>(new SnowflakeDriverException(SFError.STATEMENT_ALREADY_RUNNING_QUERY, null)));

        var exception = await Assert.ThrowsAsync<SnowflakeDbException>(() => _command.ExecuteNonQueryAsync(CancellationToken.None)).ConfigureAwait(false);

        Assert.Equal(SFError.STATEMENT_ALREADY_RUNNING_QUERY.GetErrorCode(), exception.ErrorCode);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries, Skip = PublicExceptionSkipReason)]
    [InlineData(null)]
    [InlineData("")]
    public void ExecutionWithoutCommandTextFailsWithCommandTextNotSet(string? commandText)
    {
        _command.CommandText = commandText;

        var exception = Assert.Throws<SnowflakeDbException>(() => _command.ExecuteNonQuery());

        Assert.Equal(SFError.COMMAND_TEXT_NOT_SET.GetErrorCode(), exception.ErrorCode);
        Assert.Empty(_statement.ReceivedCalls());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = PublicExceptionSkipReason)]
    public void ExecutionOnAConnectionThatIsNotOpenFailsWithExecuteCommandOnClosedConnection()
    {
        using var command = new SnowflakeDbCommand(_statement, NullLogger<SnowflakeDbCommand>.Instance)
        {
            Connection = new SnowflakeDbConnection(Substitute.For<ICoreTransport>()),
            CommandText = Sql,
        };

        var exception = Assert.Throws<SnowflakeDbException>(() => command.ExecuteNonQuery());

        Assert.Equal(SFError.EXECUTE_COMMAND_ON_CLOSED_CONNECTION.GetErrorCode(), exception.ErrorCode);
        Assert.Empty(_statement.ReceivedCalls());
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries, Skip = PublicExceptionSkipReason)]
    [InlineData(nameof(SnowflakeDbCommand.GetQueryStatus))]
    [InlineData(nameof(SnowflakeDbCommand.GetQueryStatusAsync))]
    [InlineData(nameof(SnowflakeDbCommand.GetResultsFromQueryId))]
    [InlineData(nameof(SnowflakeDbCommand.GetResultsFromQueryIdAsync))]
    public async Task MalformedQueryIdFailsWithInvalidQueryIdFormatAsync(string method)
    {
        const string malformed = "not-a-query-id";
        Func<Task> lookup = method switch
        {
            nameof(SnowflakeDbCommand.GetQueryStatus) => () => Task.FromResult(_command.GetQueryStatus(malformed)),
            nameof(SnowflakeDbCommand.GetQueryStatusAsync) => () => _command.GetQueryStatusAsync(malformed, CancellationToken.None),
            nameof(SnowflakeDbCommand.GetResultsFromQueryId) => () => Task.FromResult(_command.GetResultsFromQueryId(malformed)),
            _ => () => _command.GetResultsFromQueryIdAsync(malformed, CancellationToken.None),
        };

        var exception = await Assert.ThrowsAsync<SnowflakeDbException>(lookup).ConfigureAwait(false);

        Assert.Equal(SFError.INVALID_QUERY_ID_FORMAT.GetErrorCode(), exception.ErrorCode);
        Assert.Empty(_statement.ReceivedCalls());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = PublicExceptionSkipReason)]
    public void DriverFailureFromTheStatementReachesTheCallerAsSnowflakeDbException()
    {
        _statement.ExecuteNonQuery(default, default).ThrowsForAnyArgs(new SnowflakeDriverException(SFError.STATEMENT_ALREADY_RUNNING_QUERY, null));

        var exception = Assert.Throws<SnowflakeDbException>(() => _command.ExecuteNonQuery());

        Assert.Equal(SFError.STATEMENT_ALREADY_RUNNING_QUERY.GetErrorCode(), exception.ErrorCode);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = PublicExceptionSkipReason)]
    public void ServerFailureKeepsTheSqlStateVendorCodeAndQueryId()
    {
        _statement.ExecuteNonQuery(default, default).ThrowsForAnyArgs(new SnowflakeServerException("42000", 1003, "SQL compilation error", ValidQueryId));

        var exception = Assert.Throws<SnowflakeDbException>(() => _command.ExecuteNonQuery());

        Assert.Equal("42000", exception.SqlState);
        Assert.Equal(1003, exception.ErrorCode);
        Assert.Equal(ValidQueryId, exception.QueryId);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = PublicExceptionSkipReason)]
    public async Task CancellationSurfacesAsOperationCanceledExceptionWrappingSnowflakeDbExceptionAsync()
    {
        using var caller = new CancellationTokenSource();
        _statement.ExecuteNonQueryAsync(default, default, default).ReturnsForAnyArgs(Task.FromException<int>(
            new SnowflakeCancellationException(SFError.QUERY_CANCELLED, ValidQueryId, null)));

        var exception = await Assert.ThrowsAsync<OperationCanceledException>(
            () => _command.ExecuteNonQueryAsync(CancellationToken.None)).ConfigureAwait(false);

        Assert.Equal(caller.Token, exception.CancellationToken);
        var inner = Assert.IsType<SnowflakeDbException>(exception.InnerException);
        Assert.Equal(SFError.QUERY_CANCELLED.GetErrorCode(), inner.ErrorCode);
        Assert.Equal(ValidQueryId, inner.QueryId);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries, Skip = PublicExceptionSkipReason)]
    [InlineData(CommandType.StoredProcedure)]
    [InlineData(CommandType.TableDirect)]
    public void CommandTypeOtherThanTextIsRejected(CommandType commandType)
    {
        var exception = Assert.Throws<SnowflakeDbException>(() => _command.CommandType = commandType);

        Assert.Equal(SFError.UNSUPPORTED_FEATURE.GetErrorCode(), exception.ErrorCode);
        Assert.Equal(CommandType.Text, _command.CommandType);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = PublicExceptionSkipReason)]
    public void DesignTimeVisibleCannotBeEnabled()
    {
        var exception = Assert.Throws<SnowflakeDbException>(() => _command.DesignTimeVisible = true);

        Assert.Equal(SFError.UNSUPPORTED_FEATURE.GetErrorCode(), exception.ErrorCode);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries, Skip = PublicExceptionSkipReason)]
    [InlineData(UpdateRowSource.OutputParameters)]
    [InlineData(UpdateRowSource.FirstReturnedRecord)]
    [InlineData(UpdateRowSource.Both)]
    public void UpdatedRowSourceOtherThanNoneIsRejected(UpdateRowSource updatedRowSource)
    {
        var exception = Assert.Throws<SnowflakeDbException>(() => _command.UpdatedRowSource = updatedRowSource);

        Assert.Equal(SFError.UNSUPPORTED_FEATURE.GetErrorCode(), exception.ErrorCode);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries, Skip = PublicExceptionSkipReason)]
    public void ConnectionCannotBeReplacedOrUnset()
    {
        var original = _command.Connection;

        var replaced = Assert.Throws<SnowflakeDbException>(() => _command.Connection = OpenConnection());
        var unset = Assert.Throws<SnowflakeDbException>(() => _command.Connection = null);

        Assert.Equal(SFError.UNSUPPORTED_FEATURE.GetErrorCode(), replaced.ErrorCode);
        Assert.Equal(SFError.UNSUPPORTED_FEATURE.GetErrorCode(), unset.ErrorCode);
        Assert.Same(original, _command.Connection);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ExecuteScalarReturnsTheFirstColumnOfTheFirstRowAndClosesTheReader()
    {
        var reader = Substitute.ForPartsOf<DbDataReader>();
        reader.Read().Returns(true, true);
        reader.GetValue(0).Returns(42L, 43L);
        _statement.ExecuteReader(default).ReturnsForAnyArgs(reader);

        Assert.Equal(42L, _command.ExecuteScalar());

        reader.Received(1).Read();
        reader.Received(1).GetValue(0);
        reader.Received(1).Close();
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ExecuteScalarReturnsDbNullWhenThereAreNoRows()
    {
        var reader = Substitute.ForPartsOf<DbDataReader>();
        reader.Read().Returns(false);
        _statement.ExecuteReader(default).ReturnsForAnyArgs(reader);

        Assert.Equal(DBNull.Value, _command.ExecuteScalar());

        reader.DidNotReceiveWithAnyArgs().GetValue(default);
        reader.Received(1).Close();
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task ExecuteScalarAsyncReturnsTheFirstColumnOfTheFirstRowAsync()
    {
        var reader = Substitute.ForPartsOf<DbDataReader>();
        reader.Configure().ReadAsync(Arg.Any<CancellationToken>()).Returns(true);
        reader.GetValue(0).Returns(42L);
        _statement.ExecuteReaderAsync(default, default).ReturnsForAnyArgs(reader);

        Assert.Equal(42L, await _command.ExecuteScalarAsync(CancellationToken.None).ConfigureAwait(false));

        reader.Received(1).Close();
    }

    private static SnowflakeDbConnection OpenConnection()
    {
        var transport = Substitute.For<ICoreTransport>();
        transport.HandleMessage(default!, default!, default!).ReturnsForAnyArgs(call =>
        {
            var response = call.ArgAt<string>(1) == "connection_new"
                ? new ConnectionNewResponse { ConnHandle = new ConnectionHandle { Id = 2, Magic = 202 } }.ToByteArray()
                : [];
            return new TransportResponse(TransportResponse.CodeSuccess, new ArraySegment<byte>(response), null!);
        });
        var connection = new SnowflakeDbConnection(transport);
        connection.Open();
        return connection;
    }
}
