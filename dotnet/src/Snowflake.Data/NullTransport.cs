using Snowflake.Data.Proto;

namespace Snowflake.Data;

/// <summary>
/// A transport that always throws — used by the parameterless constructor
/// to satisfy ADO.NET factory patterns while failing fast if no real
/// transport is configured.
/// </summary>
internal sealed class NullTransport : ICoreTransport
{
    public static readonly NullTransport Instance = new();

    private NullTransport()
    {
    }

    public TransportResponse HandleMessage(string service, string method, byte[] request) =>
        throw new InvalidOperationException(
            "No transport configured. Use the constructor that accepts an ICoreTransport.");

    public Task<TransportResponse> HandleMessageAsync(
        string service, string method, byte[] request, CancellationToken cancellationToken) =>
        throw new InvalidOperationException(
            "No transport configured. Use the constructor that accepts an ICoreTransport.");
}
