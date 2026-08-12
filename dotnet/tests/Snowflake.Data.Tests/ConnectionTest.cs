#pragma warning disable CS0162 // Unreachable code detected

namespace Snowflake.Data.Tests;

[Trait("Category", "E2E")]
public sealed class ConnectionTest : IClassFixture<ITFixture>
{
    private readonly ITestOutputHelper _testOutputHelper;
    private readonly ITFixture _fixture;

    public ConnectionTest(ITestOutputHelper testOutputHelper, ITFixture fixture)
    {
        _testOutputHelper = testOutputHelper;
        _fixture = fixture;
    }

    [SnowflakeFact]
    public async Task ConnectsAsyncWithValidCredentialsAsync()
    {
        using var connection = _fixture.Factory.Create(_testOutputHelper);
        await connection.OpenAsync().ConfigureAwait(false);
        Assert.Equal(ConnectionState.Open, connection.State);
    }

    [SnowflakeFact]
    public void ConnectsWithValidCredentials()
    {
        using var connection = _fixture.Factory.Create(_testOutputHelper);
        connection.Open();
        Assert.Equal(ConnectionState.Open, connection.State);
    }

    [SnowflakeFact]
    public void CloseTransitionsToClosedState()
    {
        using var connection = _fixture.Factory.Create(_testOutputHelper);
        connection.Open();
        connection.Close();
        Assert.Equal(ConnectionState.Closed, connection.State);
    }
}
