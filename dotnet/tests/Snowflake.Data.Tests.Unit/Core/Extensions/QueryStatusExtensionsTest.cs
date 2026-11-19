using Snowflake.Data.Compatibility;
using Snowflake.Data.Core.Extensions;

namespace Snowflake.Data.Tests.Core.Extensions;

[Trait("Category", "Unit")]
public sealed class QueryStatusExtensionsTest
{
    public static TheoryData<string, QueryStatus> ServerNames => new()
    {
        { "NO_DATA", QueryStatus.NoData },
        { "RUNNING", QueryStatus.Running },
        { "ABORTING", QueryStatus.Aborting },
        { "SUCCESS", QueryStatus.Success },
        { "FAILED_WITH_ERROR", QueryStatus.FailedWithError },
        { "ABORTED", QueryStatus.Aborted },
        { "QUEUED", QueryStatus.Queued },
        { "FAILED_WITH_INCIDENT", QueryStatus.FailedWithIncident },
        { "DISCONNECTED", QueryStatus.Disconnected },
        { "RESUMING_WAREHOUSE", QueryStatus.ResumingWarehouse },
        { "QUEUED_REPARING_WAREHOUSE", QueryStatus.QueuedReparingWarehouse },
        { "RESTARTED", QueryStatus.Restarted },
        { "BLOCKED", QueryStatus.Blocked },
    };

    // Expected classification of every status: (still running, is an error).
    public static TheoryData<QueryStatus, bool, bool> Classifications => new()
    {
        { QueryStatus.NoData, true, false },
        { QueryStatus.Running, true, false },
        { QueryStatus.ResumingWarehouse, true, false },
        { QueryStatus.Queued, true, false },
        { QueryStatus.QueuedReparingWarehouse, true, false },
        { QueryStatus.Aborting, false, true },
        { QueryStatus.FailedWithError, false, true },
        { QueryStatus.Aborted, false, true },
        { QueryStatus.FailedWithIncident, false, true },
        { QueryStatus.Disconnected, false, true },
        { QueryStatus.Blocked, false, true },
        { QueryStatus.Success, false, false },
        { QueryStatus.Restarted, false, false },
    };

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [MemberData(nameof(ServerNames))]
    public void ServerNameMapsToItsStatus(string serverName, QueryStatus expected)
    {
        Assert.True(QueryStatusExtensions.TryParseServerName(serverName, out var status));
        Assert.Equal(expected, status);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData("running", QueryStatus.Running)]
    [InlineData("Failed_With_Error", QueryStatus.FailedWithError)]
    [InlineData("queued_reparing_warehouse", QueryStatus.QueuedReparingWarehouse)]
    public void ServerNameIsMatchedIgnoringCase(string serverName, QueryStatus expected)
    {
        Assert.True(QueryStatusExtensions.TryParseServerName(serverName, out var status));
        Assert.Equal(expected, status);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData("")]
    [InlineData(" ")]
    [InlineData("MYSTERY")]
    [InlineData("QUEUED_REPAIRING_WAREHOUSE")]
    [InlineData(" RUNNING")]
    [InlineData("RUNNING ")]
    [InlineData("FAILED WITH ERROR")]
    [InlineData("NoData")]
    [InlineData("1")]
    public void UnknownServerNameIsNotParsed(string serverName)
    {
        Assert.False(QueryStatusExtensions.TryParseServerName(serverName, out var status));
        Assert.Equal(default, status);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void EveryStatusHasAServerNameAndAClassification()
    {
        var mapped = ServerNames.Select(row => row.Data.Item2).ToHashSet();
        var classified = Classifications.Select(row => row.Data.Item1).ToHashSet();

        foreach (var status in EnumHelper.GetValues<QueryStatus>())
        {
            Assert.Contains(status, mapped);
            Assert.Contains(status, classified);
        }
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [MemberData(nameof(Classifications))]
    public void StatusIsClassifiedAsTheOldDriverDid(QueryStatus status, bool stillRunning, bool isAnError)
    {
        Assert.Equal(stillRunning, status.IsStillRunning());
        Assert.Equal(isAnError, status.IsAnError());
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(-1)]
    [InlineData(13)]
    [InlineData(int.MaxValue)]
    public void UndefinedStatusIsNeitherRunningNorAnError(int value)
    {
        var status = (QueryStatus)value;

        Assert.False(status.IsStillRunning());
        Assert.False(status.IsAnError());
    }
}
