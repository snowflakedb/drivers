package net.snowflake.jdbc.wiremock;

import java.io.BufferedWriter;
import java.io.File;
import java.io.IOException;
import java.io.InputStream;
import java.net.ServerSocket;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.security.SecureRandom;
import java.security.cert.X509Certificate;
import java.time.Duration;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.TimeUnit;
import javax.net.ssl.HostnameVerifier;
import javax.net.ssl.HttpsURLConnection;
import javax.net.ssl.SSLContext;
import javax.net.ssl.TrustManager;
import javax.net.ssl.X509TrustManager;
import net.snowflake.jdbc.utils.HttpTestClient;
import net.snowflake.jdbc.utils.HttpTestClient.Response;
import org.json.JSONArray;
import org.json.JSONObject;
import org.json.JSONTokener;

/**
 * Spawns the WireMock standalone JAR vendored at {@code
 * tests/wiremock/wiremock_standalone/wiremock-standalone-3.13.2.jar} as a subprocess and drives it
 * through its admin REST API. Counterpart of Python's {@code python/tests/wiremock_client.py};
 * mapping files at {@code tests/wiremock/mappings/} are shared across language wrappers.
 *
 * <p>The class is Java 8 source compatible and deliberately avoids importing {@code
 * com.github.tomakehurst.wiremock.*}. The WireMock 3.x JAR itself requires JRE 11+ to launch — test
 * classes must gate themselves with {@link #requiresJava11OrHigher()}.
 */
public final class WiremockClient implements AutoCloseable {

  private static final String WIREMOCK_JAR_RELATIVE =
      "tests/wiremock/wiremock_standalone/wiremock-standalone-3.13.2.jar";
  private static final String WIREMOCK_KEYSTORE = "wiremock-keystore.p12";
  private static final String WIREMOCK_KEYSTORE_PASSWORD = "password";
  private static final String TLS_BASE_DISABLED = "SSLv3, TLSv1, TLSv1.1";
  private static final Duration DEFAULT_HEALTH_TIMEOUT = Duration.ofSeconds(30);
  private static final Duration DEFAULT_HEALTH_POLL_INTERVAL = Duration.ofMillis(250);

  private final Path workspaceRoot;
  private final Path wiremockDir;
  private final Path wiremockJar;
  private final String tlsVersion;
  private Process process;
  private int httpPort = -1;
  private int httpsPort = -1;
  private Path securityPropsFile;
  private Thread stdoutDrain;
  private Thread stderrDrain;
  private HttpTestClient httpClient;

  public WiremockClient() {
    this(null);
  }

  /**
   * @param tlsVersion when {@code "tls12"} or {@code "tls13"}, starts an HTTPS listener restricted
   *     to that protocol version; use {@link #httpsUrl()} to connect
   */
  public WiremockClient(String tlsVersion) {
    if (tlsVersion != null && !"tls12".equals(tlsVersion) && !"tls13".equals(tlsVersion)) {
      throw new IllegalArgumentException(
          "tlsVersion must be 'tls12' or 'tls13', got: " + tlsVersion);
    }
    this.tlsVersion = tlsVersion;
    this.workspaceRoot = findWorkspaceRoot();
    this.wiremockDir = workspaceRoot.resolve("tests/wiremock");
    this.wiremockJar = workspaceRoot.resolve(WIREMOCK_JAR_RELATIVE);
    if (!Files.isRegularFile(wiremockJar)) {
      throw new IllegalStateException(
          "WireMock standalone JAR not found at "
              + wiremockJar
              + " (workspace="
              + workspaceRoot
              + ")");
    }
    if (!Files.isDirectory(wiremockDir.resolve("mappings"))) {
      throw new IllegalStateException(
          "WireMock mappings directory not found at " + wiremockDir.resolve("mappings"));
    }
  }

  /** {@code true} on JVMs new enough to launch WireMock 3.x (JRE 11+). */
  public static boolean requiresJava11OrHigher() {
    String spec = System.getProperty("java.specification.version", "");
    return !spec.startsWith("1."); // "1.8" → false, "11"/"17"/"21" → true
  }

  /** Absolute path to the WireMock test CA PEM (counterpart of Python's {@code _CA_PEM}). */
  public static Path wiremockCaPemPath() {
    return findWorkspaceRoot().resolve("tests/wiremock/wiremock-ca.pem");
  }

  public WiremockClient start() {
    if (process != null) {
      return this;
    }
    httpPort = findFreePort();
    List<String> command = new ArrayList<>();
    command.add(System.getProperty("java.home") + File.separator + "bin" + File.separator + "java");

    if (tlsVersion != null) {
      httpsPort = findFreePort();
      securityPropsFile = writeTlsSecurityProperties(tlsVersion);
      command.add("-Djava.security.properties=" + securityPropsFile.toString());
    }

    command.add("-jar");
    command.add(wiremockJar.toString());
    command.add("--root-dir");
    command.add(wiremockDir.toString());
    command.add("--enable-browser-proxying");
    command.add("--proxy-pass-through");
    command.add("false");
    command.add("--port");
    command.add(Integer.toString(httpPort));
    command.add("--disable-banner");

    if (tlsVersion != null) {
      Path keystorePath = wiremockDir.resolve(WIREMOCK_KEYSTORE);
      command.add("--https-port");
      command.add(Integer.toString(httpsPort));
      command.add("--https-keystore");
      command.add(keystorePath.toString());
      command.add("--keystore-type");
      command.add("PKCS12");
      command.add("--keystore-password");
      command.add(WIREMOCK_KEYSTORE_PASSWORD);
    }

    ProcessBuilder pb = new ProcessBuilder(command);
    pb.redirectErrorStream(false);
    try {
      process = pb.start();
    } catch (IOException e) {
      throw new RuntimeException("Failed to start WireMock process", e);
    }
    // Anything after a successful pb.start() must be guarded so a failure still reaps the child.
    try {
      stdoutDrain = drainAsync(process.getInputStream(), "wiremock-stdout");
      stderrDrain = drainAsync(process.getErrorStream(), "wiremock-stderr");
      httpClient = new HttpTestClient();
      waitForHealth(DEFAULT_HEALTH_TIMEOUT, DEFAULT_HEALTH_POLL_INTERVAL);
      if (tlsVersion != null) {
        waitForHttpsHealth(DEFAULT_HEALTH_TIMEOUT, DEFAULT_HEALTH_POLL_INTERVAL);
      }
    } catch (RuntimeException | Error e) {
      stop();
      throw e;
    }
    return this;
  }

  public String httpUrl() {
    ensureStarted();
    return "http://localhost:" + httpPort;
  }

  public String httpsUrl() {
    ensureStarted();
    if (httpsPort < 0) {
      throw new IllegalStateException("httpsUrl() requires WiremockClient(\"tls12\") or tls13");
    }
    return "https://localhost:" + httpsPort;
  }

  public void stop() {
    if (process == null) {
      return;
    }
    process.destroy();
    try {
      if (!process.waitFor(5, TimeUnit.SECONDS)) {
        process.destroyForcibly();
        process.waitFor(2, TimeUnit.SECONDS);
      }
    } catch (InterruptedException e) {
      Thread.currentThread().interrupt();
      process.destroyForcibly();
    } finally {
      process = null;
      httpPort = -1;
      httpsPort = -1;
      deleteSecurityPropsFile();
      if (stdoutDrain != null) {
        stdoutDrain.interrupt();
        stdoutDrain = null;
      }
      if (stderrDrain != null) {
        stderrDrain.interrupt();
        stderrDrain = null;
      }
      if (httpClient != null) {
        try {
          httpClient.close();
        } catch (RuntimeException ignored) {
          // Best-effort cleanup; nothing actionable.
        }
        httpClient = null;
      }
    }
  }

  @Override
  public void close() {
    stop();
  }

  public void reset() {
    Response resp = httpClient().post(adminUrl("/__admin/reset"), null);
    if (!resp.ok()) {
      throw new RuntimeException("Failed to reset WireMock: " + resp.status() + " " + resp.body());
    }
  }

  /**
   * Register a mapping file from {@code tests/wiremock/mappings/<relativePath>}. {@code
   * {{REPO_ROOT}}} is always available as a placeholder; additional placeholders may be supplied.
   * Files containing a top-level {@code "mappings": [...]} array are registered one element at a
   * time (matches Python's behaviour).
   */
  public void addMapping(String relativePath, Map<String, String> placeholders) {
    Path mappingFile = wiremockDir.resolve("mappings").resolve(relativePath);
    if (!Files.isRegularFile(mappingFile)) {
      throw new IllegalArgumentException("Mapping file not found: " + mappingFile);
    }
    String content;
    try {
      content = new String(Files.readAllBytes(mappingFile), StandardCharsets.UTF_8);
    } catch (IOException e) {
      throw new RuntimeException("Failed to read mapping " + mappingFile, e);
    }

    Map<String, String> all = new HashMap<>();
    // POSIX separators so the substituted path is valid JSON on Windows too.
    all.put("{{REPO_ROOT}}", workspaceRoot.toString().replace(File.separatorChar, '/'));
    if (placeholders != null) {
      all.putAll(placeholders);
    }
    for (Map.Entry<String, String> entry : all.entrySet()) {
      content = content.replace(entry.getKey(), entry.getValue());
    }

    JSONObject parsed = new JSONObject(new JSONTokener(content));
    if (parsed.has("mappings") && parsed.get("mappings") instanceof JSONArray) {
      JSONArray array = parsed.getJSONArray("mappings");
      for (int i = 0; i < array.length(); i++) {
        registerSingleMapping(array.getJSONObject(i).toString());
      }
    } else {
      registerSingleMapping(content);
    }
  }

  public void addMapping(String relativePath) {
    addMapping(relativePath, Collections.emptyMap());
  }

  public void addMappingJson(String mappingJson) {
    registerSingleMapping(mappingJson);
  }

  public List<JSONObject> getRequests(String urlPathPattern) {
    JSONObject body = new JSONObject().put("urlPathPattern", urlPathPattern);
    Response resp = httpClient().post(adminUrl("/__admin/requests/find"), body.toString());
    if (!resp.ok()) {
      throw new RuntimeException("Failed to query requests: " + resp.status() + " " + resp.body());
    }
    JSONObject parsed = resp.json();
    JSONArray requests = parsed.has("requests") ? parsed.getJSONArray("requests") : new JSONArray();
    List<JSONObject> out = new ArrayList<>(requests.length());
    for (int i = 0; i < requests.length(); i++) {
      out.add(requests.getJSONObject(i));
    }
    return out;
  }

  public void verifyRequestCount(int expectedCount, String urlPathPattern) {
    JSONObject body = new JSONObject().put("method", "ANY").put("urlPathPattern", urlPathPattern);
    Response resp = httpClient().post(adminUrl("/__admin/requests/count"), body.toString());
    if (!resp.ok()) {
      throw new RuntimeException(
          "Failed to query request count: " + resp.status() + " " + resp.body());
    }
    int actual = resp.json().getInt("count");
    if (actual != expectedCount) {
      throw new AssertionError(
          "Expected "
              + expectedCount
              + " requests matching '"
              + urlPathPattern
              + "', but found "
              + actual);
    }
  }

  private void ensureStarted() {
    if (process == null || httpPort < 0) {
      throw new IllegalStateException("WiremockClient is not started");
    }
  }

  private HttpTestClient httpClient() {
    ensureStarted();
    return httpClient;
  }

  private String adminUrl(String path) {
    return "http://localhost:" + httpPort + path;
  }

  private void registerSingleMapping(String mappingJson) {
    Response resp = httpClient().post(adminUrl("/__admin/mappings"), mappingJson);
    if (resp.status() != 200 && resp.status() != 201) {
      throw new RuntimeException(
          "Failed to register mapping: "
              + resp.status()
              + " "
              + resp.body()
              + " (payload="
              + mappingJson
              + ")");
    }
  }

  private void waitForHealth(Duration timeout, Duration pollInterval) {
    long deadlineNanos = System.nanoTime() + timeout.toNanos();
    long sleepMillis = Math.max(1L, pollInterval.toMillis());
    Exception lastError = null;
    while (System.nanoTime() < deadlineNanos) {
      if (!process.isAlive()) {
        throw new RuntimeException(
            "WireMock process exited prematurely with code " + process.exitValue());
      }
      try {
        Response resp = httpClient().get(adminUrl("/__admin/health"));
        if (resp.status() == 200 && resp.body().contains("\"healthy\"")) {
          return;
        }
      } catch (RuntimeException e) {
        lastError = e;
      }
      try {
        Thread.sleep(sleepMillis);
      } catch (InterruptedException e) {
        Thread.currentThread().interrupt();
        throw new RuntimeException("Interrupted while waiting for WireMock health", e);
      }
    }
    throw new RuntimeException(
        "WireMock did not become healthy within "
            + timeout
            + (lastError != null ? " (last error: " + lastError + ")" : ""));
  }

  private void waitForHttpsHealth(Duration timeout, Duration pollInterval) {
    long deadlineNanos = System.nanoTime() + timeout.toNanos();
    long sleepMillis = Math.max(1L, pollInterval.toMillis());
    Exception lastError = null;
    while (System.nanoTime() < deadlineNanos) {
      if (!process.isAlive()) {
        throw new RuntimeException(
            "WireMock process exited prematurely with code " + process.exitValue());
      }
      try {
        HttpsURLConnection connection =
            openTrustAllHttps("https://localhost:" + httpsPort + "/__admin/health");
        try {
          int status = connection.getResponseCode();
          String body = readResponseBody(connection);
          if (status == 200 && body.contains("\"healthy\"")) {
            return;
          }
        } finally {
          connection.disconnect();
        }
      } catch (IOException | RuntimeException e) {
        lastError = e;
      }
      try {
        Thread.sleep(sleepMillis);
      } catch (InterruptedException e) {
        Thread.currentThread().interrupt();
        throw new RuntimeException("Interrupted while waiting for WireMock HTTPS health", e);
      }
    }
    throw new RuntimeException(
        "WireMock HTTPS did not become healthy within "
            + timeout
            + (lastError != null ? " (last error: " + lastError + ")" : ""));
  }

  private static Path writeTlsSecurityProperties(String tlsVersion) {
    String extraDisabled = "tls12".equals(tlsVersion) ? ", TLSv1.3" : ", TLSv1.2";
    String content = "jdk.tls.disabledAlgorithms=" + TLS_BASE_DISABLED + extraDisabled + "\n";
    try {
      Path file = Files.createTempFile("wiremock-tls-", ".properties");
      try (BufferedWriter writer = Files.newBufferedWriter(file, StandardCharsets.UTF_8)) {
        writer.write(content);
      }
      return file;
    } catch (IOException e) {
      throw new RuntimeException("Failed to write TLS security properties file", e);
    }
  }

  private void deleteSecurityPropsFile() {
    if (securityPropsFile != null) {
      try {
        Files.deleteIfExists(securityPropsFile);
      } catch (IOException ignored) {
        // Best-effort cleanup.
      }
      securityPropsFile = null;
    }
  }

  private static HttpsURLConnection openTrustAllHttps(String url) throws IOException {
    TrustManager[] trustAll =
        new TrustManager[] {
          new X509TrustManager() {
            @Override
            public void checkClientTrusted(X509Certificate[] chain, String authType) {}

            @Override
            public void checkServerTrusted(X509Certificate[] chain, String authType) {}

            @Override
            public X509Certificate[] getAcceptedIssuers() {
              return new X509Certificate[0];
            }
          }
        };
    try {
      SSLContext sslContext = SSLContext.getInstance("TLS");
      sslContext.init(null, trustAll, new SecureRandom());
      HttpsURLConnection connection = (HttpsURLConnection) new URL(url).openConnection();
      connection.setSSLSocketFactory(sslContext.getSocketFactory());
      HostnameVerifier allowAll = (hostname, session) -> true;
      connection.setHostnameVerifier(allowAll);
      connection.setConnectTimeout((int) DEFAULT_HEALTH_TIMEOUT.toMillis());
      connection.setReadTimeout((int) DEFAULT_HEALTH_TIMEOUT.toMillis());
      return connection;
    } catch (Exception e) {
      throw new IOException("Failed to open trust-all HTTPS connection to " + url, e);
    }
  }

  private static String readResponseBody(HttpsURLConnection connection) throws IOException {
    InputStream stream =
        connection.getResponseCode() >= 400
            ? connection.getErrorStream()
            : connection.getInputStream();
    if (stream == null) {
      return "";
    }
    try (InputStream in = stream) {
      byte[] bytes = new byte[4096];
      int read = in.read(bytes);
      return read > 0 ? new String(bytes, 0, read, StandardCharsets.UTF_8) : "";
    }
  }

  private static int findFreePort() {
    try (ServerSocket socket = new ServerSocket(0)) {
      return socket.getLocalPort();
    } catch (IOException e) {
      throw new RuntimeException("Failed to allocate a free port", e);
    }
  }

  private static Path findWorkspaceRoot() {
    Path candidate = Paths.get(System.getProperty("user.dir")).toAbsolutePath();
    for (int i = 0; i < 6; i++) {
      if (Files.isDirectory(candidate.resolve("tests/wiremock/wiremock_standalone"))
          && Files.isDirectory(candidate.resolve("tests/wiremock/mappings"))) {
        return candidate;
      }
      Path parent = candidate.getParent();
      if (parent == null) {
        break;
      }
      candidate = parent;
    }
    throw new IllegalStateException(
        "Could not locate workspace root (expected to find tests/wiremock/wiremock_standalone/ "
            + "ascending from "
            + System.getProperty("user.dir")
            + ")");
  }

  private static Thread drainAsync(InputStream stream, String name) {
    Thread t =
        new Thread(
            () -> {
              byte[] buf = new byte[4096];
              try {
                while (stream.read(buf) != -1) {
                  // Discard — keep the pipe drained so the child doesn't block on stdout/stderr.
                }
              } catch (IOException ignored) {
                // Process exit closes the stream; nothing to do.
              }
            },
            name);
    t.setDaemon(true);
    t.start();
    return t;
  }
}
