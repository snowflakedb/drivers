package net.snowflake.perf;

import java.io.IOException;
import java.io.Writer;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.List;
// org.json is bundled in the fat jar, relocated by shadowJar (see Config).
import net.snowflake.client.jdbc.internal.org.json.JSONObject;

/** Writes the per-iteration CSV, memory-timeline CSV, and run-metadata JSON under /results. */
final class Results {

  private static final Path RESULTS_DIR = Paths.get("/results");

  private Results() {}

  static Path writeCsvResults(
      List<QueryExecution.IterationResult> results, String testName, String driverType)
      throws IOException {
    Path dir = testDir(testName, driverType);
    Files.createDirectories(dir);
    Path file = dir.resolve(testName + "_jdbc_" + driverType + "_" + epochSeconds() + ".csv");
    try (Writer w = Files.newBufferedWriter(file, StandardCharsets.UTF_8)) {
      w.write("timestamp_ms,query_s,fetch_s,row_count,cpu_time_s,peak_rss_mb\n");
      for (QueryExecution.IterationResult r : results) {
        w.write(String.format("%d,%.6f,%.6f,%d,%.6f,%.1f%n",
            r.timestampMs, r.queryTimeS, r.fetchTimeS, r.rowCount, r.cpuTimeS, r.peakRssMb));
      }
    }
    return file;
  }

  static Path writeMemoryTimeline(
      List<ResourceMonitor.Sample> timeline, String testName, String driverType)
      throws IOException {
    if (timeline.isEmpty()) {
      return null;
    }
    Path dir = testDir(testName, driverType);
    Files.createDirectories(dir);
    Path file = dir.resolve(
        "memory_timeline_" + testName + "_jdbc_" + driverType + "_" + epochSeconds() + ".csv");
    try (Writer w = Files.newBufferedWriter(file, StandardCharsets.UTF_8)) {
      w.write("timestamp_ms,rss_bytes,vm_bytes\n");
      for (ResourceMonitor.Sample s : timeline) {
        w.write(String.format("%d,%d,%d%n", s.timestampMs, s.rssBytes, s.vmBytes));
      }
    }
    return file;
  }

  static void writeRunMetadata(String driverType, String driverVersion, String serverVersion)
      throws IOException {
    Path file = RESULTS_DIR.resolve("run_metadata_jdbc_" + driverType + ".json");
    if (Files.exists(file)) {
      return;
    }
    Files.createDirectories(RESULTS_DIR);
    JSONObject metadata = new JSONObject();
    metadata.put("driver", "jdbc");
    metadata.put("driver_type", driverType);
    metadata.put("driver_version", driverVersion);
    metadata.put("runtime_language_version", System.getProperty("java.specification.version"));
    metadata.put("server_version", serverVersion);
    metadata.put("architecture", architecture());
    metadata.put("os", envOrDefault("OS_INFO", "Linux"));
    metadata.put("run_timestamp", epochSeconds());
    if ("universal".equals(driverType)) {
      metadata.put("build_rust_version", envOrDefault("BUILD_RUST_VERSION", "NA"));
    }
    Files.write(file, metadata.toString(2).getBytes(StandardCharsets.UTF_8));
  }

  private static long epochSeconds() {
    return System.currentTimeMillis() / 1000;
  }

  private static Path testDir(String testName, String driverType) {
    String subdir = testName.endsWith("_record") ? "_record" : testName;
    return RESULTS_DIR.resolve(driverType).resolve(subdir);
  }

  private static String architecture() {
    String arch = System.getProperty("os.arch", "").toLowerCase();
    if (arch.equals("amd64") || arch.equals("x64") || arch.equals("x86_64")) {
      return "x86_64";
    }
    if (arch.equals("aarch64") || arch.equals("arm64") || arch.equals("armv8")) {
      return "arm64";
    }
    return arch;
  }

  private static String envOrDefault(String key, String def) {
    String v = System.getenv(key);
    return (v == null || v.isEmpty()) ? def : v;
  }
}
