namespace Snowflake.Data.Tests;

[Trait("Category", "Unit")]
public sealed class SnowflakeDbConnectionPoolContractTest
{
    private const string GlobalSetterMessage =
        "You cannot change connection pool parameters for all the pools. Instead you can change it on a particular pool";

    private const string PreventPoolingWithoutSessionMessage =
        "Session not yet created for this connection. Unable to prevent the session from pooling";

    [SnowflakeFact]
    public void Setters_ThrowExceptionWithLegacyMessage()
    {
        var max = Assert.Throws<Exception>(() => SnowflakeDbConnectionPool.SetMaxPoolSize(1));
        var timeout = Assert.Throws<Exception>(() => SnowflakeDbConnectionPool.SetTimeout(1));
        var pooling = Assert.Throws<Exception>(() => SnowflakeDbConnectionPool.SetPooling(false));

        Assert.IsType<Exception>(max);
        Assert.IsType<Exception>(timeout);
        Assert.IsType<Exception>(pooling);
        Assert.Equal(GlobalSetterMessage, max.Message);
        Assert.Equal(GlobalSetterMessage, timeout.Message);
        Assert.Equal(GlobalSetterMessage, pooling.Message);
    }

    [SnowflakeFact]
    public void PreventPooling_WithoutSession_ThrowsExceptionWithLegacyMessage()
    {
        using var connection = new SnowflakeDbConnection();

        var thrown = Assert.Throws<Exception>(() => connection.PreventPooling());

        Assert.IsType<Exception>(thrown);
        Assert.Equal(PreventPoolingWithoutSessionMessage, thrown.Message);
    }
}
