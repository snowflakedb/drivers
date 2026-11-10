using Snowflake.Data.Proto;

namespace Snowflake.Data.Interfaces;

internal interface IConnectionHandleWrapper
{
    ConnectionHandle? ConnHandle { get; }
}
