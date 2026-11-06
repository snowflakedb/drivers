namespace Snowflake.Data.Tests;

[Trait("Category", "E2E")]
public class ConnectionLifecycleTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    public ConnectionLifecycleTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    [SnowflakeFact]
    public void IsOpen_ReturnsTrueAfterOpen()
    {
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();
        Assert.True(connection.IsOpen());
        Assert.Equal(ConnectionState.Open, connection.State);
    }

    [SnowflakeFact]
    public async Task IsOpen_ReturnsTrueAfterOpenAsync()
    {
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);
        Assert.True(connection.IsOpen());
        Assert.Equal(ConnectionState.Open, connection.State);
    }

    [SnowflakeFact]
    public void IsOpen_ReturnsFalseAfterClose()
    {
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();
        connection.Close();
        Assert.False(connection.IsOpen());
        Assert.Equal(ConnectionState.Closed, connection.State);
    }

    [SnowflakeFact]
    public async Task CloseAsync_ClosesAnOpenConnectionAsync()
    {
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);
        await connection.CloseAsync(CancellationToken.None).ConfigureAwait(false);
        Assert.False(connection.IsOpen());
        Assert.Equal(ConnectionState.Closed, connection.State);
    }

    [SnowflakeFact]
    public async Task CloseAsync_AfterSyncClose_LeavesConnectionClosedAsync()
    {
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);
        connection.Close();
        await connection.CloseAsync(CancellationToken.None).ConfigureAwait(false);
        Assert.False(connection.IsOpen());
        Assert.Equal(ConnectionState.Closed, connection.State);
    }

    [SnowflakeFact]
    public async Task CloseAsync_DoesNotCloseWhenCancellationAlreadyRequestedAsync()
    {
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);
        using var cts = new CancellationTokenSource();
        cts.Cancel();

        try
        {
            await connection.CloseAsync(cts.Token).ConfigureAwait(false);
        }
        catch (OperationCanceledException)
        {
        }

        Assert.True(connection.IsOpen());
        Assert.Equal(ConnectionState.Open, connection.State);
    }
}
