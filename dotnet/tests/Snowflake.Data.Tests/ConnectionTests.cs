#pragma warning disable CS0162 // Unreachable code detected

namespace Snowflake.Data.Tests;

[Trait("Category", "E2E")]
public sealed class ConnectionTests : IClassFixture<ITFixture>
{
    private readonly ITestOutputHelper _testOutputHelper;

    public ConnectionTests(ITestOutputHelper testOutputHelper)
    {
        _testOutputHelper = testOutputHelper;
    }

    [SnowflakeFact]
    public async Task ConnectsAsyncWithValidCredentials()
    {
        using var connection = TestConnectionFactory.Create(_testOutputHelper);
        await connection.OpenAsync();
        Assert.Equal(ConnectionState.Open, connection.State);
    }

    [SnowflakeFact]
    public void ConnectsWithValidCredentials()
    {
        using var connection = TestConnectionFactory.Create(_testOutputHelper);
        connection.Open();
        Assert.Equal(ConnectionState.Open, connection.State);
    }

    [SnowflakeFact]
    public void CloseTransitionsToClosedState()
    {
        using var connection = TestConnectionFactory.Create(_testOutputHelper);
        connection.Open();
        connection.Close();
        Assert.Equal(ConnectionState.Closed, connection.State);
    }
}
