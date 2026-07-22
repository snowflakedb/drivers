package net.snowflake.perf;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.sql.Connection;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * PUT/GET file-transfer perf test: times {@code statement.execute(sql)} (no row fetch). Mirrors
 * {@code drivers/python/app/put_execution.py} and {@code drivers/odbc/app/put_execution.cpp}.
 */
final class PutExecution {

  private static final Pattern FILE_URI = Pattern.compile("file://([^\\s]+)");

  static final class IterationResult {
    final long timestampMs;
    final double queryTimeS;
    final double cpuTimeS;
    final double peakRssMb;

    IterationResult(long timestampMs, double queryTimeS, double cpuTimeS, double peakRssMb) {
      this.timestampMs = timestampMs;
      this.queryTimeS = queryTimeS;
      this.cpuTimeS = cpuTimeS;
      this.peakRssMb = peakRssMb;
    }
  }

  static final class PutTestOutput {
    final List<IterationResult> results;
    final List<ResourceMonitor.Sample> memoryTimeline;

    PutTestOutput(List<IterationResult> results, List<ResourceMonitor.Sample> memoryTimeline) {
      this.results = results;
      this.memoryTimeline = memoryTimeline;
    }
  }

  private PutExecution() {}

  static PutTestOutput execute(
      Connection conn, String sql, int warmupIterations, int iterations) throws SQLException {
    System.out.println("Query: " + sql);
    try (Statement stmt = conn.createStatement()) {
      for (int i = 0; i < warmupIterations; i++) {
        executePutGet(stmt, sql);
      }

      ResourceMonitor monitor = new ResourceMonitor(100);
      monitor.start();
      List<IterationResult> results = new ArrayList<>();
      for (int i = 0; i < iterations; i++) {
        results.add(executePutGet(stmt, sql));
      }
      List<ResourceMonitor.Sample> timeline = monitor.stop();

      System.out.printf("Completed %d PUT/GET iterations%n", results.size());
      return new PutTestOutput(results, timeline);
    }
  }

  private static IterationResult executePutGet(Statement stmt, String sql) throws SQLException {
    createGetTargetDirectory(sql);

    double cpuStart = Common.processCpuSeconds();
    long queryStart = System.nanoTime();
    stmt.execute(sql);
    double queryTimeS = (System.nanoTime() - queryStart) / 1_000_000_000.0;
    double cpuTimeS = Common.processCpuSeconds() - cpuStart;

    return new IterationResult(
        System.currentTimeMillis(), queryTimeS, cpuTimeS, Common.peakRssMb());
  }

  /**
   * For GET commands, clear and recreate the local target directory so each iteration starts clean
   * (mirrors {@code _create_get_target_directory} in the python/odbc apps).
   */
  private static void createGetTargetDirectory(String sql) {
    if (!sql.trim().toUpperCase().startsWith("GET")) {
      return;
    }
    Matcher matcher = FILE_URI.matcher(sql);
    if (!matcher.find()) {
      return;
    }
    Path target = Paths.get(matcher.group(1));
    try {
      if (Files.exists(target)) {
        try (var walk = Files.walk(target)) {
          walk.sorted(Comparator.reverseOrder()).forEach(PutExecution::deleteQuietly);
        }
      }
      Files.createDirectories(target);
    } catch (IOException e) {
      System.out.println("Warning: could not prepare GET target directory: " + e.getMessage());
    }
  }

  private static void deleteQuietly(Path p) {
    try {
      Files.delete(p);
    } catch (IOException ignored) {
      // best effort
    }
  }
}
