using System.Net.Http;
using System.Text;
using System.Text.Json;
using Snowflake.Data.Tests.Wiremock;

namespace Snowflake.Data.Tests.Reference.Wiremock;

public static class ReferenceWiremock
{
    // LegacySnowflakeConnection runs this statement after every old-driver open.
    private const string LegacyFacadeResultFormatMapping = "query/alter_session_dotnet_query_result_format_success.json";

    public static string BuildHttpConnectionString(
        WiremockClient wiremock,
        params string[] extraSettings)
    {
        if (wiremock.HttpPort < 0)
            throw new InvalidOperationException("WireMock has not been started.");

        var parts = new List<string>
        {
            "account=test_account",
            "user=test_user",
            "password=test_password",
            $"server_url=http://localhost:{wiremock.HttpPort}",
            "host=localhost",
            $"port={wiremock.HttpPort}",
            "scheme=http",
        };
        parts.AddRange(extraSettings);
        return string.Join(";", parts);
    }

    public static Task<WiremockClient> StartWithMappingsAsync(
        CancellationToken cancellationToken,
        params string[] mappingRelativePaths) =>
        StartWithMappingsAsync(new WiremockClient(), cancellationToken, mappingRelativePaths);

    /// <summary>
    /// Same as <see cref="StartWithMappingsAsync(CancellationToken, string[])"/>, with an additional
    /// TLS 1.2 listener on <see cref="WiremockClient.HttpsPort"/> whose certificate chains to
    /// <see cref="WiremockClient.WiremockCaPemPath"/>.
    /// </summary>
    public static Task<WiremockClient> StartHttpsWithMappingsAsync(
        CancellationToken cancellationToken,
        params string[] mappingRelativePaths) =>
        StartWithMappingsAsync(new WiremockClient("tls12"), cancellationToken, mappingRelativePaths);

    public static async Task<IReadOnlyList<RecordedRequest>> GetRecordedRequestsAsync(
        WiremockClient wiremock,
        CancellationToken cancellationToken)
    {
        using var http = new HttpClient();
        using var response = await http.GetAsync(AdminUri(wiremock, "requests"), cancellationToken).ConfigureAwait(false);
        response.EnsureSuccessStatusCode();
        var json = await response.Content.ReadAsStringAsync().ConfigureAwait(false);
        using var document = JsonDocument.Parse(json);
        return document.RootElement.GetProperty("requests").EnumerateArray()
            .Select(entry => entry.GetProperty("request"))
            .Select(request => new RecordedRequest(
                request.GetProperty("method").GetString()!,
                request.GetProperty("url").GetString()!,
                request.TryGetProperty("body", out var body) ? body.GetString() ?? string.Empty : string.Empty))
            .ToList();
    }

    public static async Task<int> CountQueryRequestsAsync(
        WiremockClient wiremock,
        string bodyRegex,
        CancellationToken cancellationToken)
    {
        var pattern = JsonSerializer.Serialize(new
        {
            method = "POST",
            urlPathPattern = "/queries/v1/query-request.*",
            bodyPatterns = new[] { new { matches = bodyRegex } },
        });
        using var http = new HttpClient();
        using var content = new StringContent(pattern, Encoding.UTF8, "application/json");
        using var response = await http.PostAsync(AdminUri(wiremock, "requests/count"), content, cancellationToken)
            .ConfigureAwait(false);
        response.EnsureSuccessStatusCode();
        var body = await response.Content.ReadAsStringAsync().ConfigureAwait(false);
        using var document = JsonDocument.Parse(body);
        return document.RootElement.GetProperty("count").GetInt32();
    }

    private static async Task<WiremockClient> StartWithMappingsAsync(
        WiremockClient wiremock,
        CancellationToken cancellationToken,
        string[] mappingRelativePaths)
    {
        try
        {
            await wiremock.StartAsync(cancellationToken: cancellationToken).ConfigureAwait(false);
            await RemovePreloadedMappingsAsync(wiremock, cancellationToken).ConfigureAwait(false);
            await wiremock.AddMappingAsync(LegacyFacadeResultFormatMapping, cancellationToken: cancellationToken)
                .ConfigureAwait(false);
            foreach (var mapping in mappingRelativePaths)
                await wiremock.AddMappingAsync(mapping, cancellationToken: cancellationToken).ConfigureAwait(false);
            return wiremock;
        }
        catch
        {
            wiremock.Dispose();
            throw;
        }
    }

    // WireMock's --root-dir loads every file under tests/wiremock/mappings at startup, and higher-priority
    // stubs there would shadow the mappings a test registers. WireMock 3.13 answers a bulk
    // DELETE /__admin/mappings with 500 while stubs from multi-mapping files are loaded, so stubs are
    // deleted one by one.
    private static async Task RemovePreloadedMappingsAsync(WiremockClient wiremock, CancellationToken cancellationToken)
    {
        using var http = new HttpClient { Timeout = TimeSpan.FromSeconds(10) };
        using var listing = await http.GetAsync(AdminUri(wiremock, "mappings"), cancellationToken).ConfigureAwait(false);
        listing.EnsureSuccessStatusCode();
        var json = await listing.Content.ReadAsStringAsync().ConfigureAwait(false);
        using var document = JsonDocument.Parse(json);
        foreach (var mapping in document.RootElement.GetProperty("mappings").EnumerateArray())
        {
            var id = mapping.GetProperty("id").GetString();
            using var deleted = await http.DeleteAsync(AdminUri(wiremock, $"mappings/{id}"), cancellationToken)
                .ConfigureAwait(false);
            deleted.EnsureSuccessStatusCode();
        }
    }

    private static Uri AdminUri(WiremockClient wiremock, string path) =>
        new($"http://localhost:{wiremock.HttpPort}/__admin/{path}");
}
