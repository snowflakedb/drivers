package net.snowflake.jdbc.wiremock;

import java.io.File;
import java.io.IOException;
import java.io.InputStream;
import java.net.ServerSocket;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.time.Duration;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.TimeUnit;
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
  private static final Duration DEFAULT_HEALTH_TIMEOUT = Duration.ofSeconds(30);
  private static final Duration DEFAULT_HEALTH_POLL_INTERVAL = Duration.ofMillis(250);

  private final Path workspaceRoot;
  private final Path wiremockDir;
  private final Path wiremockJar;
  private Process process;
  private int httpPort = -1;
  private Thread stdoutDrain;
  private Thread stderrDrain;
  private HttpTestClient httpClient;

  public WiremockClient() {
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

  public WiremockClient start() {
    if (process != null) {
      return this;
    }
    httpPort = findFreePort();
    List<String> command = new ArrayList<>();
    command.add(System.getProperty("java.home") + File.separator + "bin" + File.separator + "java");
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
