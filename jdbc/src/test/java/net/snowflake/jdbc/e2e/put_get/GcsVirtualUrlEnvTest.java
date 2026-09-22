package net.snowflake.jdbc.e2e.put_get;

import static net.snowflake.jdbc.utils.TestParameters.buildJdbcUrl;
import static net.snowflake.jdbc.utils.TestParameters.loadDefaultConnectionProperties;
import static net.snowflake.jdbc.utils.TestParameters.withDefaultAuth;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assertions.fail;

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
import java.util.Properties;
import java.util.UUID;
import java.util.concurrent.TimeUnit;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.jdbc.utils.DriverCompatibility;
import net.snowflake.jdbc.utils.EnabledOnGCP;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

/**
 * Smoke test that SQL PUT/GET still succeed when {@code
 * SNOWFLAKE_GCS_FORCE_VIRTUAL_STYLE_DOMAINS=true}. URL shape for that flag is covered by {@code
 * sf_core} unit tests; this class only checks the transfer still works on a GCS stage. The variable
 * is process-global, so PUT/GET run in a child JVM (same pattern as {@code TroubleshootingTests}).
 */
public class GcsVirtualUrlEnvTest {

  private static final String FORCE_VIRTUAL_STYLE_DOMAINS =
      "SNOWFLAKE_GCS_FORCE_VIRTUAL_STYLE_DOMAINS";
  private static final String PAYLOAD = "gcs virtual-host payload";
  private static final long WORKER_TIMEOUT_MINUTES = 5;

  @EnabledOnGCP
  @Test
  public void shouldPutAndGetOnGcsWhenVirtualStyleDomainsForced(@TempDir Path tmpDir)
      throws Exception {
    // Given a child JVM with SNOWFLAKE_GCS_FORCE_VIRTUAL_STYLE_DOMAINS=true and a local file
    Path uploadDir = Files.createDirectories(tmpDir.resolve("upload"));
    Path downloadDir = Files.createDirectories(tmpDir.resolve("download"));
    Path source = uploadDir.resolve("virtual_url.txt");
    Files.write(source, (PAYLOAD + "\n").getBytes(StandardCharsets.UTF_8));
    Path workerLog = tmpDir.resolve("worker.log");

    ProcessBuilder pb = new ProcessBuilder(buildWorkerCommand(source, downloadDir));
    pb.environment().put(FORCE_VIRTUAL_STYLE_DOMAINS, "true");
    pb.redirectErrorStream(true);
    pb.redirectOutput(workerLog.toFile());

    // When PUT then GET run against a GCS stage in that JVM
    Process process = pb.start();
    if (!process.waitFor(WORKER_TIMEOUT_MINUTES, TimeUnit.MINUTES)) {
      process.destroyForcibly();
      process.waitFor(10, TimeUnit.SECONDS);
      fail(
          "Worker JVM timed out after "
              + WORKER_TIMEOUT_MINUTES
              + " minutes:\n"
              + readWorkerLog(workerLog));
    }

    // Then the transfer succeeds and the downloaded bytes match
    assertEquals(0, process.exitValue(), "Worker JVM failed:\n" + readWorkerLog(workerLog));

    Path downloaded = downloadDir.resolve("virtual_url.txt");
    assertTrue(Files.exists(downloaded), "Expected the downloaded file on disk");

    // TODO(SNOW-4143382): BD#70 — on this path the old driver writes the still-encrypted bytes
    // and reports DOWNLOADED, so only the new driver is held to the round-trip. Drop the guard
    // once the reference driver is bumped to a release carrying the fix.
    if (DriverCompatibility.isNewDriver()) {
      String contents = new String(Files.readAllBytes(downloaded), StandardCharsets.UTF_8).trim();
      assertEquals(PAYLOAD, contents, "Unexpected downloaded content");
    }
  }

  public static void main(String[] args) throws Exception {
    if (args.length != 2) {
      System.err.println("Usage: GcsVirtualUrlEnvTest <source-file> <download-dir>");
      System.exit(2);
    }
    String env = System.getenv(FORCE_VIRTUAL_STYLE_DOMAINS);
    if (!"true".equalsIgnoreCase(env)) {
      System.err.println("Expected " + FORCE_VIRTUAL_STYLE_DOMAINS + "=true, got: " + env);
      System.exit(2);
    }

    Path source = Paths.get(args[0]);
    Path downloadDir = Paths.get(args[1]);
    Files.createDirectories(downloadDir);

    Properties props = withDefaultAuth(loadDefaultConnectionProperties());
    Class.forName(SnowflakeDriver.class.getName());
    try (Connection connection = DriverManager.getConnection(buildJdbcUrl(props), props);
        Statement statement = connection.createStatement()) {
      useDatabaseAndSchema(statement, props);
      String stageName =
          ("TEST_GCS_VIRTUAL_URL_" + UUID.randomUUID().toString().replace("-", "")).toUpperCase();
      statement.execute("CREATE TEMPORARY STAGE IF NOT EXISTS " + stageName);

      String putSql =
          String.format(
              "PUT '%s' @%s AUTO_COMPRESS=FALSE OVERWRITE=TRUE", fileUri(source), stageName);
      try (ResultSet putRs = statement.executeQuery(putSql)) {
        if (!putRs.next()) {
          System.err.println("PUT returned no rows");
          System.exit(1);
        }
        String status = putRs.getString("status");
        if (putRs.wasNull() || !"UPLOADED".equals(status)) {
          System.err.println("Expected PUT UPLOADED, got: " + status);
          System.exit(1);
        }
      }

      String getSql =
          String.format("GET @%s/virtual_url.txt '%s/'", stageName, fileUri(downloadDir));
      try (ResultSet getRs = statement.executeQuery(getSql)) {
        if (!getRs.next()) {
          System.err.println("GET returned no rows");
          System.exit(1);
        }
        String status = getRs.getString("status");
        if (getRs.wasNull() || !"DOWNLOADED".equals(status)) {
          System.err.println("Expected GET DOWNLOADED, got: " + status);
          System.exit(1);
        }
      }
    }
  }

  private static String readWorkerLog(Path workerLog) throws Exception {
    if (!Files.exists(workerLog)) {
      return "";
    }
    return new String(Files.readAllBytes(workerLog), StandardCharsets.UTF_8);
  }

  private static void useDatabaseAndSchema(Statement statement, Properties props) throws Exception {
    String database = props.getProperty("db");
    String schema = props.getProperty("schema");
    if (database != null && !database.isEmpty()) {
      statement.execute("use database " + database);
    }
    if (schema != null && !schema.isEmpty()) {
      statement.execute("use schema " + schema);
    }
  }

  private static String fileUri(Path path) {
    return "file://" + path.toAbsolutePath().toString().replace('\\', '/');
  }

  private static List<String> buildWorkerCommand(Path source, Path downloadDir) {
    List<String> cmd = new ArrayList<>();
    cmd.add(Paths.get(System.getProperty("java.home"), "bin", "java").toString());
    for (String arg : ManagementFactory.getRuntimeMXBean().getInputArguments()) {
      if (arg.startsWith("--add-opens") || arg.startsWith("--enable-native-access")) {
        cmd.add(arg);
      }
    }
    cmd.add("-cp");
    cmd.add(System.getProperty("java.class.path"));
    cmd.add(GcsVirtualUrlEnvTest.class.getName());
    cmd.add(source.toAbsolutePath().toString());
    cmd.add(downloadDir.toAbsolutePath().toString());
    return cmd;
  }
}
