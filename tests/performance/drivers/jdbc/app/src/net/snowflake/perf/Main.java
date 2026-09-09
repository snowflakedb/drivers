package net.snowflake.perf;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.List;
import java.util.Properties;

/** Runs the configured SELECT, PUT/GET, concurrent, or cold-start test and writes metrics. */
public final class Main {

  private Main() {}

  public static void main(String[] args) {
    Config config;
    try {
      config = new Config();
    } catch (Exception e) {
      System.out.println("ERROR: " + e.getMessage());
      System.exit(1);
      return;
    }

    if ("cold_start".equals(config.testType)) {
      try {
        if ("1".equals(System.getenv("COLD_START_CHILD"))) {
          runColdStartChild(config);
        } else {
          runColdStartParent(config);
        }
      } catch (Exception e) {
        System.out.println("Cold start failed: " + e.getMessage());
        e.printStackTrace();
        System.exit(1);
      }
      return;
    }

    // recorded-HTTP (select_recorded_http) reuses the select path.
    if (!"select".equals(config.testType)
        && !"put_get".equals(config.testType)
        && !"concurrent".equals(config.testType)) {
      System.out.println(
          "ERROR: jdbc perf supports test_type=select, put_get, concurrent, or cold_start (got "
              + config.testType
              + ")");
      System.exit(1);
    }

    Properties props = config.connectionProperties();
    String url = config.jdbcUrl(props);

    Connection conn = null;
    try {
      conn = ConnectionFactory.connect(config.driverType, url, props);
      String driverVersion = ConnectionFactory.driverVersion(conn);
      ConnectionFactory.executeSetupQueries(conn, config.setupQueries());

      Path csv;
      List<ResourceMonitor.Sample> memoryTimeline;
      if ("put_get".equals(config.testType)) {
        PutExecution.PutTestOutput output =
            PutExecution.execute(
                conn, config.sqlCommand, config.warmupIterations, config.iterations);
        csv = Results.writePutGetCsvResults(output.results, config.testName, config.driverType);
        memoryTimeline = output.memoryTimeline;
      } else if ("concurrent".equals(config.testType)) {
        QueryExecution.FetchTestOutput output =
            ConcurrentExecution.execute(
                config.driverType,
                url,
                props,
                config.setupQueries(),
                config.sqlCommand,
                config.warmupIterations,
                config.iterations,
                config.workerCount);
        csv = Results.writeCsvResults(output.results, config.testName, config.driverType);
        memoryTimeline = output.memoryTimeline;
      } else {
        QueryExecution.FetchTestOutput output =
            QueryExecution.executeFetchTest(
                conn, config.sqlCommand, config.warmupIterations, config.iterations);
        csv = Results.writeCsvResults(output.results, config.testName, config.driverType);
        memoryTimeline = output.memoryTimeline;
      }

      String serverVersion =
          "true".equals(System.getenv("WIREMOCK_REPLAY"))
              ? "N/A"
              : ConnectionFactory.serverVersion(conn);
      Results.writeRunMetadata(config.driverType, driverVersion, serverVersion);

      Results.writeMemoryTimeline(memoryTimeline, config.testName, config.driverType);
      System.out.println("Complete: " + csv);
    } catch (Exception e) {
      System.out.println("Test failed: " + e.getMessage());
      e.printStackTrace();
      System.exit(1);
    } finally {
      if (conn != null) {
        try {
          conn.close();
        } catch (Exception ignored) {
          // ignore
        }
      }
    }
  }

  private static void runColdStartParent(Config config) throws Exception {
    System.out.println("\n=== Cold-Start Test (" + config.iterations + " iterations) ===");
    List<String> rows = new ArrayList<>();
    for (int i = 0; i < config.iterations; i++) {
      String label = "iter " + (i + 1) + "/" + config.iterations;
      ProcessBuilder pb = new ProcessBuilder(childCommand());
      pb.environment().put("COLD_START_CHILD", "1");
      pb.redirectError(ProcessBuilder.Redirect.INHERIT);
      Process proc = pb.start();
      String stdout;
      try (BufferedReader reader =
          new BufferedReader(new InputStreamReader(proc.getInputStream(), StandardCharsets.UTF_8))) {
        StringBuilder buf = new StringBuilder();
        String line;
        while ((line = reader.readLine()) != null) {
          if (buf.length() > 0) {
            buf.append('\n');
          }
          buf.append(line);
        }
        stdout = buf.toString();
      }
      int code = proc.waitFor();
      if (code != 0) {
        throw new IllegalStateException("[" + label + "] child failed (exit " + code + ")");
      }
      String[] lines = stdout.trim().split("\n");
      if (lines.length == 0 || lines[lines.length - 1].isEmpty()) {
        throw new IllegalStateException("[" + label + "] no child output");
      }
      String row = lines[lines.length - 1];
      System.out.println("  [" + label + "] " + row);
      rows.add(row);
    }
    Path csv = Results.writeColdStartCsv(rows, config.testName, config.driverType);
    Results.writeRunMetadata(config.driverType, "UNKNOWN", "N/A");
    System.out.println("Complete: " + csv);
  }

  private static void runColdStartChild(Config config) throws Exception {
    long t0 = System.nanoTime();
    Properties props = config.connectionProperties();
    String url = config.jdbcUrl(props);
    long t1;
    long t2;
    try (Connection conn = ConnectionFactory.connect(config.driverType, url, props)) {
      t1 = System.nanoTime();
      try (Statement stmt = conn.createStatement();
          ResultSet rs = stmt.executeQuery(config.sqlCommand)) {
        if (!rs.next()) {
          throw new IllegalStateException("SELECT 1 returned no row");
        }
        int value = rs.getInt(1);
        if (value != 1) {
          throw new IllegalStateException("Expected 1, got " + value);
        }
      }
      t2 = System.nanoTime();
    }
    double connectS = (t1 - t0) / 1e9;
    double select1S = (t2 - t1) / 1e9;
    double e2eS = (t2 - t0) / 1e9;
    // Print after close so driver shutdown logs cannot steal the last-line CSV row.
    System.out.printf(
        "%d,%.6f,%.6f,%.6f,%.6f,%.1f%n",
        System.currentTimeMillis(),
        e2eS,
        connectS,
        select1S,
        Common.processCpuSeconds(),
        Common.peakRssMb());
  }

  private static List<String> childCommand() {
    List<String> cmd = new ArrayList<>();
    cmd.add(System.getProperty("java.home") + "/bin/java");
    String opts = System.getenv("JDBC_JAVA_OPTS");
    if (opts != null) {
      for (String part : opts.split("\\s+")) {
        if (!part.isEmpty()) {
          cmd.add(part);
        }
      }
    }
    cmd.add("-cp");
    cmd.add(System.getProperty("java.class.path"));
    cmd.add("net.snowflake.perf.Main");
    return cmd;
  }
}
