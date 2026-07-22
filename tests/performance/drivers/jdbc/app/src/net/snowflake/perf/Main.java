package net.snowflake.perf;

import java.nio.file.Path;
import java.sql.Connection;
import java.util.List;
import java.util.Properties;

/** Runs the configured SELECT or PUT/GET test against Snowflake and writes metrics to /results. */
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

    // Phase 3: select + put_get. recorded-HTTP (select_recorded_http) reuses the select path.
    if (!"select".equals(config.testType) && !"put_get".equals(config.testType)) {
      System.out.println(
          "ERROR: jdbc perf supports test_type=select or put_get (got " + config.testType + ")");
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
}
