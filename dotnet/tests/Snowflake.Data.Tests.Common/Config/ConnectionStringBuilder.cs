namespace Snowflake.Data.Tests.Config;

public sealed class ConnectionStringBuilder : IConnectionStringBuilder
{
    private string? _account;
    private string? _user;
    private string? _password;
    private string? _warehouse;
    private string? _database;
    private string? _schema;
    private string? _role;
    private string? _pat;

    private string _authenticator = string.Empty;

    public IConnectionStringBuilder WithAccount(string? account)
    {
        if (account != null)
            _account = account;

        return this;
    }

    public IConnectionStringBuilder WithUser(string? user)
    {
        if (user != null)
            _user = user;
        return this;
    }

    public IConnectionStringBuilder WithPassword(string? password)
    {
        if (password != null)
            _password = password;
        return this;
    }

    public IConnectionStringBuilder WithWarehouse(string? warehouse)
    {
        if (warehouse != null)
            _warehouse = warehouse;

        return this;
    }

    public IConnectionStringBuilder WithDatabase(string? database)
    {
        if (database != null)
            _database = database;

        return this;
    }

    public IConnectionStringBuilder WithSchema(string? schema)
    {
        if (schema != null)
            _schema = schema;

        return this;
    }

    public IConnectionStringBuilder WithRole(string? role)
    {
        if (role != null)
            _role = role;

        return this;
    }

    public IConnectionStringBuilder WithPat(string? pat)
    {
        if (pat == null)
            return this;

        _authenticator = "programmatic_access_token";
        _pat = pat;
        return this;
    }

    public string Build()
    {
        List<string> keys = [];
        if (_account != null)
            keys.Add($"{nameof(_account).Substring(1)}={_account}");

        if (_user != null)
            keys.Add($"{nameof(_user).Substring(1)}={_user}");

        if (_password != null)
            keys.Add($"{nameof(_password).Substring(1)}={_password}");

        if (_warehouse != null)
            keys.Add($"{nameof(_warehouse).Substring(1)}={_warehouse}");

        if (_database != null)
        {
            keys.Add($"{nameof(_database).Substring(1)}={_database}");
            keys.Add($"db={_database}");
        }

        if (_schema != null)
            keys.Add($"{nameof(_schema).Substring(1)}={_schema}");

        if (_role != null)
            keys.Add($"{nameof(_role).Substring(1)}={_role}");

        if (_pat != null)
            keys.Add($"token={_pat}");

        if ("programmatic_access_token".Equals(_authenticator, StringComparison.InvariantCultureIgnoreCase))
            keys.Add("authenticator=PROGRAMMATIC_ACCESS_TOKEN");
        else if ("snowflake".Equals(_authenticator, StringComparison.InvariantCultureIgnoreCase))
            keys.Add("authenticator=snowflake");
        else if ("snowflake_jwt".Equals(_authenticator, StringComparison.InvariantCultureIgnoreCase))
            keys.Add("authenticator=snowflake_jwt");
        else if ("snowflake_password".Equals(_authenticator, StringComparison.InvariantCultureIgnoreCase))
            keys.Add("authenticator=snowflake_password");
        else if ("username_password_mfa".Equals(_authenticator, StringComparison.InvariantCultureIgnoreCase))
            keys.Add("authenticator=username_password_mfa");
        else if ("externalbrowser".Equals(_authenticator, StringComparison.InvariantCultureIgnoreCase))
            keys.Add("authenticator=externalbrowser");
        else if ("oauth".Equals(_authenticator, StringComparison.InvariantCultureIgnoreCase))
            keys.Add("authenticator=oauth");
        else if ("oauth_client_credentials".Equals(_authenticator, StringComparison.InvariantCultureIgnoreCase))
            keys.Add("authenticator=oauth_client_credentials");
        else if ("oauth_authorization_code".Equals(_authenticator, StringComparison.InvariantCultureIgnoreCase))
            keys.Add("authenticator=oauth_authorization_code");
        else if ("workload_identity".Equals(_authenticator, StringComparison.InvariantCultureIgnoreCase))
            keys.Add("authenticator=workload_identity");
        else if ("snowflake".Equals(_authenticator, StringComparison.InvariantCultureIgnoreCase))
            keys.Add("authenticator=snowflake");
        else if (_authenticator.StartsWith("https://"))
            throw new NotImplementedException("TODO Okta SSO");

        return string.Join(";", keys);
    }
}
