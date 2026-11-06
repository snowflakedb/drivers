using System.Data;
using System.Data.Common;
using System.Threading;
using System.Threading.Tasks;

namespace Snowflake.Data;

public interface ISnowflakeDbConnection : IDbConnection
{
    Task OpenAsync(CancellationToken cancellationToken = default);

    new DbCommand CreateCommand();

    bool IsOpen();

    Task CloseAsync(CancellationToken cancellationToken);

    bool IsStillRunning(QueryStatus status);

    bool IsAnError(QueryStatus status);
}
