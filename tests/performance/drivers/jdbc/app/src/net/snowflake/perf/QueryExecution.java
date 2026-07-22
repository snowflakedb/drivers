package net.snowflake.perf;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.List;

final class QueryExecution {

  static final class IterationResult {
    final long timestampMs;
    final double queryTimeS;
    final double fetchTimeS;
    final long rowCount;
    final double cpuTimeS;
    final double peakRssMb;

    IterationResult(long timestampMs, double queryTimeS, double fetchTimeS, long rowCount,
        double cpuTimeS, double peakRssMb) {
      this.timestampMs = timestampMs;
      this.queryTimeS = queryTimeS;
      this.fetchTimeS = fetchTimeS;
      this.rowCount = rowCount;
      this.cpuTimeS = cpuTimeS;
      this.peakRssMb = peakRssMb;
    }
  }

  static final class FetchTestOutput {
    final List<IterationResult> results;
    final List<ResourceMonitor.Sample> memoryTimeline;

    FetchTestOutput(List<IterationResult> results, List<ResourceMonitor.Sample> memoryTimeline) {
      this.results = results;
      this.memoryTimeline = memoryTimeline;
    }
  }

  private QueryExecution() {}

  static FetchTestOutput executeFetchTest(
      Connection conn, String sql, int warmupIterations, int iterations) throws SQLException {
    System.out.println("Query: " + sql);
    try (Statement stmt = conn.createStatement()) {
      for (int i = 0; i < warmupIterations; i++) {
        executeQuery(stmt, sql);
      }

      ResourceMonitor monitor = new ResourceMonitor(100);
      monitor.start();
      List<IterationResult> results = new ArrayList<>();
      for (int i = 0; i < iterations; i++) {
        results.add(executeQuery(stmt, sql));
      }
      List<ResourceMonitor.Sample> timeline = monitor.stop();

      validateRowCounts(results);
      return new FetchTestOutput(results, timeline);
    }
  }

  private static IterationResult executeQuery(Statement stmt, String sql) throws SQLException {
    long queryStart = System.nanoTime();
    try (ResultSet rs = stmt.executeQuery(sql)) {
      double queryTimeS = (System.nanoTime() - queryStart) / 1_000_000_000.0;

      double cpuStart = Common.processCpuSeconds();
      long fetchStart = System.nanoTime();
      int columnCount = rs.getMetaData().getColumnCount();
      long rowCount = 0;
      while (rs.next()) {
        for (int c = 1; c <= columnCount; c++) {
          rs.getObject(c); // materialize every column
        }
        rowCount++;
      }
      double fetchTimeS = (System.nanoTime() - fetchStart) / 1_000_000_000.0;
      double cpuTimeS = Common.processCpuSeconds() - cpuStart;
      return new IterationResult(System.currentTimeMillis(), queryTimeS, fetchTimeS, rowCount,
          cpuTimeS, Common.peakRssMb());
    }
  }

  private static void validateRowCounts(List<IterationResult> results) {
    if (results.isEmpty()) {
      return;
    }
    long expected;
    int startIdx;
    String expectedFromRecording = System.getenv("EXPECTED_ROW_COUNT");
    if (expectedFromRecording != null && !expectedFromRecording.isEmpty()) {
      expected = Long.parseLong(expectedFromRecording);
      startIdx = 0;
    } else {
      expected = results.get(0).rowCount; // first iteration is the baseline
      startIdx = 1;
    }
    if (expected == 0) {
      throw new IllegalStateException("Row count baseline is 0 (likely a silent query failure)");
    }
    for (int i = startIdx; i < results.size(); i++) {
      long actual = results.get(i).rowCount;
      if (actual != expected) {
        throw new IllegalStateException(
            String.format("Row count mismatch at iteration %d: %d != %d", i, actual, expected));
      }
    }
    System.out.printf("All %d iterations returned %d rows%n", results.size(), expected);
  }
}
