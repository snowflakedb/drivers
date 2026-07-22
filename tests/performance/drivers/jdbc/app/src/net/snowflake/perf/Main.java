package net.snowflake.perf;

import java.nio.file.Path;
import java.sql.Connection;
import java.util.Properties;

/** Runs the configured SELECT test against Snowflake and writes CSV/JSON metrics to /results. */
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

    // Phase 1: universal driver + SELECT only.
    if (!"universal".equals(config.driverType) || !"select".equals(config.testType)) {
      System.out.println(
          "ERROR: jdbc perf supports only driver_type=universal + test_type=select (got "
              + config.driverType + "/" + config.testType + ")");
      System.exit(1);
    }

    Properties props = config.connectionProperties();
    String url = config.jdbcUrl(props);

    Connection conn = null;
    try {
      conn = ConnectionFactory.connect(config.driverType, url, props);
      String driverVersion = ConnectionFactory.driverVersion(conn);
      ConnectionFactory.executeSetupQueries(conn, config.setupQueries());

      QueryExecution.FetchTestOutput output =
          QueryExecution.executeFetchTest(
              conn, config.sqlCommand, config.warmupIterations, config.iterations);

      String serverVersion =
          "true".equals(System.getenv("WIREMOCK_REPLAY"))
              ? "N/A"
              : ConnectionFactory.serverVersion(conn);
      Results.writeRunMetadata(config.driverType, driverVersion, serverVersion);

      Path csv = Results.writeCsvResults(output.results, config.testName, config.driverType);
      Results.writeMemoryTimeline(output.memoryTimeline, config.testName, config.driverType);
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
