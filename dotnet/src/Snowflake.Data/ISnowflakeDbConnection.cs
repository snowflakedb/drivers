using System.Data;
using System.Data.Common;
using System.Security;
using System.Threading;
using System.Threading.Tasks;
using Snowflake.Data.Core;


namespace Snowflake.Data;

public interface ISnowflakeDbConnection : IDbConnection
{
    SecureString Password { get; set; }

    SecureString Passcode { get; set; }

    SecureString OAuthClientSecret { get; set; }

    SecureString Token { get; set; }

    Task OpenAsync(CancellationToken cancellationToken = default);

    new DbCommand CreateCommand();

    bool IsOpen();

    void PreventPooling();

    Task CloseAsync(CancellationToken cancellationToken);

    bool IsStillRunning(QueryStatus status);

    bool IsAnError(QueryStatus status);
}
