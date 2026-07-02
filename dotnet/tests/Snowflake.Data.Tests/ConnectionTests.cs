using System.Data;
using Microsoft.Testing.Platform.Extensions.Messages;
using Xunit;
#pragma warning disable CS0162 // Unreachable code detected

namespace Snowflake.Data.Tests;

[Trait("Category", "E2E")]
public sealed class ConnectionTests
{
    [Fact]
    public async Task ConnectsWithValidCredentials()
    {
        await using var connection = TestConnectionFactory.Create();
        await connection.OpenAsync(CancellationToken.None);
        Assert.Equal(ConnectionState.Open, connection.State);
    }

    [Fact]
    public async Task CloseTransitionsToClosedState()
    {
        await using var connection = TestConnectionFactory.Create();
        await connection.OpenAsync(CancellationToken.None);
        await connection.CloseAsync();
        Assert.Equal(ConnectionState.Closed, connection.State);
    }
}
