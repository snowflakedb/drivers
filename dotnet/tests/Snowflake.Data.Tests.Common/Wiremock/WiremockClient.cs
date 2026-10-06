using System.Diagnostics;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Text.Json;
using Snowflake.Data.Tests.Compatibility;

#if NETFRAMEWORK
using System.Net.Http;
#endif

namespace Snowflake.Data.Tests.Wiremock;

/// <summary>
/// Spawns the WireMock standalone JAR vendored at
/// <c>tests/wiremock/wiremock_standalone/wiremock-standalone-3.13.2.jar</c> as a subprocess
/// and drives it through its admin REST API.
/// </summary>
public sealed class WiremockClient : IDisposable
{
    private const string WiremockJarRelative = "tests/wiremock/wiremock_standalone/wiremock-standalone-3.13.2.jar";
    private const string WiremockKeystore = "wiremock-keystore.p12";
    private const string WiremockKeystorePassword = "password";
    private static readonly TimeSpan DefaultHealthTimeout = TimeSpan.FromSeconds(90);
    private static readonly TimeSpan DefaultHealthPollInterval = TimeSpan.FromMilliseconds(250);

    private readonly string? _tlsVersion;
    private readonly string _workspaceRoot;
    private readonly string _wiremockDir;
    private readonly string _wiremockJar;
    private Process? _process;
    private int _httpPort = -1;
    private string? _securityPropsFile;
    private HttpClient? _httpClient;

    /// <param name="tlsVersion">
    /// When <c>"tls12"</c> or <c>"tls13"</c>, starts an HTTPS listener restricted
    /// to that protocol version; use https url to connect.
    /// </param>
    public WiremockClient(string? tlsVersion = null)
    {
        if (tlsVersion is not null and not ("tls12" or "tls13"))
            throw new ArgumentException($"tlsVersion must be 'tls12' or 'tls13', got: {tlsVersion}", nameof(tlsVersion));

        _tlsVersion = tlsVersion;
        _workspaceRoot = FindWorkspaceRoot();
        _wiremockDir = Path.Combine(_workspaceRoot, "tests", "wiremock");
        _wiremockJar = Path.Combine(_workspaceRoot, WiremockJarRelative.Replace('/', Path.DirectorySeparatorChar));

        if (!File.Exists(_wiremockJar))
            throw new InvalidOperationException($"WireMock standalone JAR not found at {_wiremockJar} (workspace={_workspaceRoot})");
        if (!Directory.Exists(Path.Combine(_wiremockDir, "mappings")))
            throw new InvalidOperationException($"WireMock mappings directory not found at {Path.Combine(_wiremockDir, "mappings")}");
    }

    public string WiremockCaPemPath => Path.GetFullPath(Path.Combine(_wiremockDir, "wiremock-ca.pem"));

    public int HttpsPort { get; private set; } = -1;

    public async Task<WiremockClient> StartAsync(DataReceivedEventHandler? onDataReceived = null,
        DataReceivedEventHandler? onErrReceived = null, CancellationToken cancellationToken = default)
    {
        if (_process is not null)
            return this;

        _httpPort = FindFreePort();

        var args = new List<string>();

        if (_tlsVersion is not null)
        {
            HttpsPort = FindFreePort();
            _securityPropsFile = await GetTlsSecurityPropertiesFileAsync(_tlsVersion, cancellationToken).ConfigureAwait(false);
            args.Add($"-Djava.security.properties={_securityPropsFile}");
        }

        args.AddRange([
            "-jar", _wiremockJar,
            "--root-dir", _wiremockDir,
            "--enable-browser-proxying",
            "--proxy-pass-through", "false",
            "--port", _httpPort.ToString(),
            "--disable-banner"
        ]);

        if (_tlsVersion is not null)
        {
            var keystorePath = Path.Combine(_wiremockDir, WiremockKeystore);
            args.AddRange([
                "--https-port", HttpsPort.ToString(),
                "--https-keystore", keystorePath,
                "--keystore-type", "PKCS12",
                "--keystore-password", WiremockKeystorePassword
            ]);
        }

        var psi = new ProcessStartInfo
        {
            FileName = FindJava(),
            UseShellExecute = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            CreateNoWindow = true,
        };
#if !NETFRAMEWORK
        foreach (var arg in args)
            psi.ArgumentList.Add(arg);
#else
        psi.Arguments = BuildArguments(args);
#endif

        _process = Process.Start(psi)
            ?? throw new InvalidOperationException("Failed to start WireMock process");

        try
        {
            _process.OutputDataReceived += onDataReceived ?? ((_, _) => { });
            _process.ErrorDataReceived += onErrReceived ?? ((_, _) => { });
            _process.BeginOutputReadLine();
            _process.BeginErrorReadLine();

            var handler = new HttpClientHandler
            {
                ServerCertificateCustomValidationCallback = HttpClientHandler.DangerousAcceptAnyServerCertificateValidator
            };
            _httpClient = new HttpClient(handler) { Timeout = TimeSpan.FromSeconds(10) };
            await WaitForHealthAsync(DefaultHealthTimeout, DefaultHealthPollInterval, $"http://localhost:{_httpPort}", cancellationToken).ConfigureAwait(false);

            if (_tlsVersion is not null)
            {
                await WaitForHealthAsync(DefaultHealthTimeout, DefaultHealthPollInterval, $"https://localhost:{HttpsPort}", cancellationToken).ConfigureAwait(false);
            }
        }
        catch
        {
            Stop();
            throw;
        }

        return this;
    }

    public async Task AddMappingAsync(string relativePath, Dictionary<string, string>? placeholders = null, CancellationToken cancellationToken = default)
    {
        var mappingFile = Path.Combine(_wiremockDir, "mappings", relativePath.Replace('/', Path.DirectorySeparatorChar));
        if (!File.Exists(mappingFile))
            throw new ArgumentException($"Mapping file not found: {mappingFile}");

        var content = await FileShims.ReadAllTextAsync(mappingFile).ConfigureAwait(false);
        content = content.Replace("{{REPO_ROOT}}", _workspaceRoot.Replace(Path.DirectorySeparatorChar, '/'));
        if (placeholders is not null)
        {
            foreach (var kvp in placeholders)
                content = content.Replace(kvp.Key, kvp.Value);
        }

        using var doc = JsonDocument.Parse(content);
        if (doc.RootElement.TryGetProperty("mappings", out var mappingsArray)
            && mappingsArray.ValueKind == JsonValueKind.Array)
        {
            foreach (var mapping in mappingsArray.EnumerateArray())
                await RegisterSingleMappingAsync(mapping.GetRawText()).ConfigureAwait(false);
        }
        else
        {
            await RegisterSingleMappingAsync(content).ConfigureAwait(false);
        }
    }

    private void Stop()
    {
        if (_process is null)
            return;

        try
        {
            _process.Kill();
            _process.WaitForExit(5000);
        }
        catch
        {
            // Best-effort cleanup
        }
        finally
        {
            _process.Dispose();
            _process = null;
            _httpPort = -1;
            HttpsPort = -1;

            if (_securityPropsFile is not null)
            {
                try
                { File.Delete(_securityPropsFile); }
                catch { /* best-effort */ }
                _securityPropsFile = null;
            }

            _httpClient?.Dispose();
            _httpClient = null;
        }
    }

    public void Dispose() => Stop();

    private async Task RegisterSingleMappingAsync(string mappingJson, CancellationToken cancellationToken = default)
    {
        if (_process is null || _httpPort < 0)
            throw new InvalidOperationException("WiremockClient is not started");

        using var content = new StringContent(mappingJson, Encoding.UTF8, "application/json");

        if (_process is null || _httpPort < 0)
            throw new InvalidOperationException("WiremockClient is not started");

        using var response = await _httpClient!.PostAsync($"http://localhost:{_httpPort}/__admin/mappings", content, cancellationToken).ConfigureAwait(false);

        if ((int)response.StatusCode is 200 or 201)
            return;

        var body = await response.Content.ReadAsStringAsync().ConfigureAwait(false);
        throw new InvalidOperationException($"Failed to register mapping: {(int)response.StatusCode} {body} (payload={mappingJson})");
    }

    private async Task WaitForHealthAsync(TimeSpan timeout, TimeSpan pollInterval, string basePath, CancellationToken cancellationToken)
    {
        var deadline = DateTime.UtcNow + timeout;
        Exception? lastError = null;
        while (DateTime.UtcNow < deadline)
        {
            if (_process is { HasExited: true })
                throw new InvalidOperationException(
                    $"WireMock process exited prematurely with code {_process.ExitCode}");

            try
            {
                using var response = await _httpClient!.GetAsync($"{basePath}/__admin/health", cancellationToken).ConfigureAwait(false);
                if (response.IsSuccessStatusCode)
                {
                    var body = await response.Content.ReadAsStringAsync().ConfigureAwait(false);
                    if (body.Contains("\"healthy\""))
                        return;
                }
            }
            catch (Exception e)
            {
                lastError = e;
            }

            await Task.Delay(pollInterval, cancellationToken).ConfigureAwait(false);
        }

        var suffix = (lastError is not null ? $" (last error: {lastError.Message})" : "");
        throw new InvalidOperationException($"WireMock did not become healthy within {timeout}{suffix}");
    }

    private static async Task<string> GetTlsSecurityPropertiesFileAsync(string tlsVersion, CancellationToken _ = default)
    {
        var extraDisabled = tlsVersion == "tls12" ? "TLSv1.3" : "TLSv1.2";
        var content = $"jdk.tls.disabledAlgorithms=SSLv3, TLSv1, TLSv1.1, {extraDisabled}\n";
        var path = Path.Combine(Path.GetTempPath(), $"wiremock-tls-{Guid.NewGuid():N}.properties");
        await FileShims.WriteAllTextAsync(path, content).ConfigureAwait(false);
        return path;
    }

    private static int FindFreePort()
    {
        var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        var port = ((IPEndPoint)listener.LocalEndpoint).Port;
        listener.Stop();
        return port;
    }

    private static string FindJava()
    {
        var javaHome = Environment.GetEnvironmentVariable("JAVA_HOME");
        if (string.IsNullOrEmpty(javaHome))
            return "java";

        var javaPath = Path.Combine(javaHome, "bin", "java");
        if (File.Exists(javaPath) || File.Exists(javaPath + ".exe"))
            return javaPath;

        return "java";
    }

    private static string FindWorkspaceRoot()
    {
        var candidate = Path.GetFullPath(Directory.GetCurrentDirectory());
        for (; ; )
        {
            if (Directory.Exists(Path.Combine(candidate, "tests", "wiremock", "wiremock_standalone"))
                && Directory.Exists(Path.Combine(candidate, "tests", "wiremock", "mappings")))
                return candidate;

            var parent = Directory.GetParent(candidate)?.FullName;
            if (parent is null)
                break;

            if (Directory.Exists(Path.Combine(candidate, "dotnet")))
                break;

            candidate = parent;
        }

        throw new InvalidOperationException($"Could not locate workspace root (expected to find tests/wiremock/wiremock_standalone/ ascending from {Directory.GetCurrentDirectory()})");
    }

#if NETFRAMEWORK
    private static string BuildArguments(List<string> args)
    {
        var sb = new StringBuilder();
        foreach (var arg in args)
        {
            if (sb.Length > 0) sb.Append(' ');
            if (arg.Contains(' ') || arg.Contains('"'))
                sb.Append('"').Append(arg.Replace("\"", "\\\"")).Append('"');
            else
                sb.Append(arg);
        }
        return sb.ToString();
    }
#endif
}
