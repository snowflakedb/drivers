namespace Snowflake.Data;

// FIPS_TLS_STATUS_WITHHELD: withheld from the shipped contract while the status shape is still under consideration.
// public sealed class SnowflakeDbDriver
// {
//     private readonly IDatabaseDriverService _driver;
//
//     public SnowflakeDbDriver()
//         : this(SfCoreTransport.Instance)
//     {
//     }
//
//     internal SnowflakeDbDriver(ICoreTransport transport)
//     {
//         _driver = new DatabaseDriverServiceClient(transport);
//     }
//
//     public SnowflakeTlsStatus GetTlsStatus()
//     {
//         var response = _driver.DriverGetTlsStatus(new DriverGetTlsStatusRequest());
//         return new SnowflakeTlsStatus(response.TlsProviderIsFips, response.FipsBuildEnabled);
//     }
// }
