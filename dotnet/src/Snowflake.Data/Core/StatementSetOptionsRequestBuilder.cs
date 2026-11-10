using Snowflake.Data.Proto;

namespace Snowflake.Data.Core;

/// <summary>
/// Collects the sf_core statement options of one <see cref="StatementSetOptionsRequest"/>.
/// </summary>
/// <remarks>
/// A value equal to sf_core's default for a fresh statement adds no entry, so <see cref="IsEmpty"/>
/// tells the caller when the request can be skipped.
/// </remarks>
internal sealed class StatementSetOptionsRequestBuilder
{
    // TODO(SNOW-4429197): more sf_core statement option keys follow, starting with multi_statement_count for multi-statement execution.
    private const string AsyncExecutionOption = "async_execution";
    private const string QueryTagOption = "query_tag";

    private readonly StatementSetOptionsRequest _request;

    /// <summary>Starts an empty request for <paramref name="handle"/>.</summary>
    internal StatementSetOptionsRequestBuilder(StatementHandle handle)
    {
        _request = new StatementSetOptionsRequest { StmtHandle = handle };
    }

    /// <summary>True when no option has been added.</summary>
    internal bool IsEmpty => _request.Options.Count == 0;

    /// <summary>Tags the statement's query with <paramref name="queryTag"/>; <c>null</c> adds no entry.</summary>
    internal StatementSetOptionsRequestBuilder WithQueryTag(string? queryTag)
    {
        if (queryTag is not null)
            _request.Options[QueryTagOption] = new ConfigSetting { StringValue = queryTag };
        return this;
    }

    /// <summary>
    /// Makes sf_core submit the query without waiting for it to finish; <c>false</c> adds no entry.
    /// sf_core runs PUT and GET in blocking mode even when this is set.
    /// </summary>
    internal StatementSetOptionsRequestBuilder WithAsyncExecution(bool enabled)
    {
        if (enabled)
            _request.Options[AsyncExecutionOption] = new ConfigSetting { BoolValue = true };
        return this;
    }

    /// <summary>The request holding every option added so far.</summary>
    internal StatementSetOptionsRequest Build() => _request;
}
