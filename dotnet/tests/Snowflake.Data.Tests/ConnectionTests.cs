using System.Data;
using Xunit;
#pragma warning disable CS0162 // Unreachable code detected

namespace Snowflake.Data.Tests;

[Trait("Category", "E2E")]
public sealed class ConnectionTests
{
    [Fact]
    public void ConnectsWithValidCredentials()
    {
        using var connection = TestConnectionFactory.Create();
        connection.Open();
        Assert.Equal(ConnectionState.Open, connection.State);
    }

    [Fact]
    public void CloseTransitionsToClosedState()
    {
        using var connection = TestConnectionFactory.Create();
        connection.Open();
        connection.Close();
        Assert.Equal(ConnectionState.Closed, connection.State);
    }
}
