package net.snowflake.jdbc.e2e.logging;

import static net.snowflake.jdbc.utils.TestParameters.buildJdbcUrl;
import static net.snowflake.jdbc.utils.TestParameters.loadDefaultConnectionProperties;
import static net.snowflake.jdbc.utils.TestParameters.withDefaultAuth;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.lang.management.ManagementFactory;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Properties;
import java.util.stream.Stream;
import net.snowflake.jdbc.utils.SkipOldDriver;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

@SkipOldDriver("SNOWFLAKE_TROUBLESHOOTING_ENABLED is universal-driver only")
class TroubleshootingTests {

  /** sf_core login path; emitted at DEBUG and filtered at default wrapper/core levels. */
  private static final String DEBUG_LOGIN_MARKER = "Login successful, extracting session tokens";

  /**
   * SNOWFLAKE_TROUBLESHOOTING_ENABLED is read once at native LogManager init (process start). A
   * child JVM is forked with the env vars set so they are visible before the first connection.
   */
  @Test
  void shouldCreateTroubleshootingLogFileWhenEnabledViaEnvironmentVariable(@TempDir Path tmpDir)
      throws Exception {
    // Given SNOWFLAKE_TROUBLESHOOTING_ENABLED is set to "true" and
    // SNOWFLAKE_TROUBLESHOOTING_REPORT_PATH points to a temporary directory
    List<String> cmd = buildWorkerCommand(tmpDir);
    ProcessBuilder pb = new ProcessBuilder(cmd);
    Map<String, String> env = pb.environment();
    env.put("SNOWFLAKE_TROUBLESHOOTING_ENABLED", "true");
    env.put("SNOWFLAKE_TROUBLESHOOTING_REPORT_PATH", tmpDir.toString());
    for (String key : new String[] {"CORE_PATH", "PARAMETER_PATH", "SF_TEST_BROWSER_OPENER"}) {
      String val = System.getenv(key);
      if (val != null) {
        env.put(key, val);
      }
    }
    pb.redirectErrorStream(true);

    // When a connection is established and a query is executed
    Process p = pb.start();
    byte[] output = drain(p.getInputStream());
    int exitCode = p.waitFor();
    assertEquals(0, exitCode, "Worker JVM failed:\n" + new String(output, StandardCharsets.UTF_8));

    // Then a troubleshooting log file exists in the configured directory
    Path logFile = tmpDir.resolve("sf_driver_troubleshooting.log");
    assertTrue(
        Files.exists(logFile),
        "Expected sf_driver_troubleshooting.log in " + tmpDir + ", found: " + listDir(tmpDir));

    // And the log file contains debug-level entries below the configured log level
    String contents = new String(Files.readAllBytes(logFile), StandardCharsets.UTF_8);
    assertFalse(contents.isEmpty(), "Troubleshooting log file is empty");
    assertTrue(
        contents.contains(DEBUG_LOGIN_MARKER),
        "Expected debug-level login event in troubleshooting log, got: "
            + contents.substring(0, Math.min(contents.length(), 500)));
  }

  /** Worker entry point: connect, execute SELECT 1, exit. */
  public static void main(String[] args) throws Exception {
    Properties props = withDefaultAuth(loadDefaultConnectionProperties());
    props.setProperty("tracing", "OFF");
    try (Connection conn = DriverManager.getConnection(buildJdbcUrl(props), props);
        Statement stmt = conn.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT 1")) {
      if (!rs.next()) {
        System.err.println("Expected one row from SELECT 1");
        System.exit(1);
      }
    }
  }

  private static List<String> buildWorkerCommand(Path tmpDir) {
    List<String> cmd = new ArrayList<>();
    cmd.add(Paths.get(System.getProperty("java.home"), "bin", "java").toString());
    for (String arg : ManagementFactory.getRuntimeMXBean().getInputArguments()) {
      // Forward module-access flags the native JDBC bridge needs in the child JVM.
      if (arg.startsWith("--add-opens") || arg.startsWith("--enable-native-access")) {
        cmd.add(arg);
      }
    }
    cmd.add("-cp");
    cmd.add(System.getProperty("java.class.path"));
    cmd.add(TroubleshootingTests.class.getName());
    cmd.add(tmpDir.toString());
    return cmd;
  }

  private static byte[] drain(InputStream is) throws IOException {
    ByteArrayOutputStream out = new ByteArrayOutputStream();
    byte[] buf = new byte[1024];
    int n;
    while ((n = is.read(buf)) != -1) {
      out.write(buf, 0, n);
    }
    return out.toByteArray();
  }

  private static String listDir(Path dir) {
    try (Stream<Path> stream = Files.list(dir)) {
      return String.join(", ", stream.map(p -> p.getFileName().toString()).toArray(String[]::new));
    } catch (IOException e) {
      return "<error listing dir: " + e.getMessage() + ">";
    }
  }
}
