namespace Snowflake.Data.Tests.Config;

public interface IConnectionStringBuilder
{
    string Build();
    IConnectionStringBuilder WithAccount(string? account);
    IConnectionStringBuilder WithUser(string? user);
    IConnectionStringBuilder WithPassword(string? password);
    IConnectionStringBuilder WithWarehouse(string? warehouse);
    IConnectionStringBuilder WithDatabase(string? database);
    IConnectionStringBuilder WithSchema(string? schema);
    IConnectionStringBuilder WithRole(string? role);
    IConnectionStringBuilder WithPat(string? pat);
}
