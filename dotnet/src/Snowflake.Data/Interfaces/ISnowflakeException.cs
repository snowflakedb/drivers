namespace Snowflake.Data.Interfaces;

internal interface ISnowflakeException
{
    /// <summary>
    /// Builds the public exception thrown to the client.
    /// </summary>
    Exception ToPublicException();
}
