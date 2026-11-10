using System.Data.Common;

namespace Snowflake.Data;

// TODO SNOW-4382591 this is just a PoC to get foot into the door. It will be completely overhauled. It's just to settle contract.
public sealed class SnowflakeDbException : DbException
{
    public SnowflakeDbException(
        string sqlState,
        int vendorCode,
        string errorMessage,
        string queryId)
    {
        throw new NotImplementedException("TODO next milestone");
    }

    public SnowflakeDbException(SFError error, string queryId, Exception innerException)
    {
        throw new NotImplementedException("TODO next milestone");
    }

    public SnowflakeDbException(SFError error, params object[] args)
    {
        throw new NotImplementedException("TODO next milestone");
    }

    public SnowflakeDbException(string sqlState, SFError error, params object[] args)
    {
        throw new NotImplementedException("TODO next milestone");
    }

    public SnowflakeDbException(Exception innerException, SFError error, params object[] args)
    {
        throw new NotImplementedException("TODO next milestone");
    }

    public SnowflakeDbException(
        Exception innerException,
        string sqlState,
        SFError error,
        params object[] args)
    {
        throw new NotImplementedException("TODO next milestone");
    }

#pragma warning disable CS0108, CS0114
    public string SqlState => throw new NotImplementedException("TODO next milestone");
#pragma warning restore CS0108, CS0114

    public string QueryId
    {
        get => throw new NotImplementedException("TODO next milestone");
        set => throw new NotImplementedException("TODO next milestone");
    }

    public override int ErrorCode => throw new NotImplementedException("TODO next milestone");
}
