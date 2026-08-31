using Snowflake.Data.Core.Session;
// ReSharper disable UnusedType.Global
// ReSharper disable UnusedMember.Global

namespace Snowflake.Data;

public sealed class SnowflakeDbSessionPool
{
    public bool GetPooling() => throw new NotImplementedException(
        "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");

    public int GetMinPoolSize() => throw new NotImplementedException(
        "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");

    public int GetMaxPoolSize() => throw new NotImplementedException(
        "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");

    public int GetCurrentPoolSize() => throw new NotImplementedException(
        "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");

    public long GetExpirationTimeout() => throw new NotImplementedException(
        "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");

    public long GetConnectionTimeout() => throw new NotImplementedException(
        "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");

    public long GetWaitForIdleSessionTimeout() => throw new NotImplementedException(
        "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");

    public void ClearPool() => throw new NotImplementedException(
        "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");

    public ChangedSessionBehavior GetChangedSession() => throw new NotImplementedException(
        "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");
}
