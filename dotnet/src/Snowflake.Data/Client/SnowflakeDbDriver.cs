using Snowflake.Data.Interop;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Client;

public sealed class SnowflakeDbDriver
{
    private readonly IDatabaseDriverService _driver;

    public SnowflakeDbDriver()
        : this(SfCoreTransport.Instance)
    {
    }

    internal SnowflakeDbDriver(ICoreTransport transport)
    {
        _driver = new DatabaseDriverServiceClient(transport);
    }

    public SnowflakeFipsStatus GetFipsStatus()
    {
        var response = _driver.DriverGetFipsStatus(new DriverGetFipsStatusRequest());
        return new SnowflakeFipsStatus(response.IsFips);
    }
}
