using Snowflake.Data.Core;
using Snowflake.Data.Proto;
using Snowflake.Data.Tests.Discovery;

namespace Snowflake.Data.Tests.Core;

[Trait("Category", "Unit")]
public sealed class StatementSetOptionsRequestBuilderTest
{
    private static readonly StatementHandle Handle = new() { Id = 7, Magic = 42 };

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void BuilderWithoutOptionsIsEmpty()
    {
        var builder = new StatementSetOptionsRequestBuilder(Handle);

        Assert.True(builder.IsEmpty);
        var request = builder.Build();
        Assert.Equal(Handle, request.StmtHandle);
        Assert.Empty(request.Options);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void NullQueryTagAndDisabledAsyncExecutionAddNoEntries()
    {
        var builder = new StatementSetOptionsRequestBuilder(Handle)
            .WithQueryTag(null)
            .WithAsyncExecution(false);

        Assert.True(builder.IsEmpty);
        Assert.Empty(builder.Build().Options);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void EmptyQueryTagIsSentAsAnEmptyString()
    {
        var request = new StatementSetOptionsRequestBuilder(Handle).WithQueryTag(string.Empty).Build();

        var entry = Assert.Single(request.Options);
        Assert.Equal("query_tag", entry.Key);
        Assert.Equal(ConfigSetting.ValueOneofCase.StringValue, entry.Value.ValueCase);
        Assert.Equal(string.Empty, entry.Value.StringValue);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void QueryTagAndAsyncExecutionAreBothInOneRequest()
    {
        var builder = new StatementSetOptionsRequestBuilder(Handle)
            .WithQueryTag("nightly-load")
            .WithAsyncExecution(true);

        Assert.False(builder.IsEmpty);
        var request = builder.Build();
        Assert.Equal(Handle, request.StmtHandle);
        Assert.Equal(2, request.Options.Count);
        Assert.Equal(new ConfigSetting { StringValue = "nightly-load" }, request.Options["query_tag"]);
        Assert.Equal(new ConfigSetting { BoolValue = true }, request.Options["async_execution"]);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void LaterQueryTagReplacesEarlierOne()
    {
        var request = new StatementSetOptionsRequestBuilder(Handle)
            .WithQueryTag("first")
            .WithQueryTag("second")
            .Build();

        var entry = Assert.Single(request.Options);
        Assert.Equal("second", entry.Value.StringValue);
    }
}
