namespace Snowflake.Data.Tests.Config;

public interface IConnectionStringBuilder
{
    IConnectionStringBuilderNoAuth WithAccount(string? account);
    IConnectionStringBuilderNoAuth WithUser(string? user);
    IConnectionStringBuilderNoAuth WithWarehouse(string? warehouse);
    IConnectionStringBuilderNoAuth WithDatabase(string? database);
    IConnectionStringBuilderNoAuth WithSchema(string? schema);
    IConnectionStringBuilderNoAuth WithRole(string? role);
}

public interface IConnectionStringBuilderNoAuth : IConnectionStringBuilder
{
    IConnectionStringBuilderAuth WithPat(string? pat);
    IConnectionStringBuilderAuth WithKeyFile(string? keyFile);
    IConnectionStringBuilderAuth WithPassword(string? password);
}

public interface IConnectionStringBuilderAuth : IConnectionStringBuilder
{
    IConnectionStringBuilderAuth WithTokenFilePath(string? tokenFilePath);
    IConnectionStringBuilderAuth WithExplicitlySetAuthenticator(string authenticator);
    string Build();
}
