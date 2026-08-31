// TODO
// ReSharper disable UnusedType.Global
// ReSharper disable UnusedMember.Global
namespace Snowflake.Data;

public interface ISnowflakeCredentialManager
{
    string GetCredentials(string key);

    void RemoveCredentials(string key);

    void SaveCredentials(string key, string token);
}
