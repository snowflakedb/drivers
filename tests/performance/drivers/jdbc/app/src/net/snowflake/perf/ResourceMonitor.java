package net.snowflake.perf;

import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.List;

/** Samples RSS/VmSize from {@code /proc/self/status} on a daemon thread (Linux only). */
final class ResourceMonitor {

  static final class Sample {
    final long timestampMs;
    final long rssBytes;
    final long vmBytes;

    Sample(long timestampMs, long rssBytes, long vmBytes) {
      this.timestampMs = timestampMs;
      this.rssBytes = rssBytes;
      this.vmBytes = vmBytes;
    }
  }

  private static final boolean IS_LINUX =
      System.getProperty("os.name", "").toLowerCase().contains("linux");
  private static final Path PROC_STATUS = Paths.get("/proc/self/status");

  private final long intervalMs;
  private final List<Sample> samples = new ArrayList<>();
  private volatile boolean stop = false;
  private Thread thread;

  ResourceMonitor(long intervalMs) {
    this.intervalMs = intervalMs;
  }

  void start() {
    if (!IS_LINUX) {
      return;
    }
    samples.clear();
    stop = false;
    thread = new Thread(() -> {
      while (!stop) {
        sampleOnce();
        try {
          Thread.sleep(intervalMs);
        } catch (InterruptedException e) {
          Thread.currentThread().interrupt();
          break;
        }
      }
      sampleOnce();
    }, "mem-monitor");
    thread.setDaemon(true);
    thread.start();
  }

  List<Sample> stop() {
    stop = true;
    if (thread != null) {
      try {
        thread.join(2000);
      } catch (InterruptedException e) {
        Thread.currentThread().interrupt();
      }
    }
    return new ArrayList<>(samples);
  }

  private void sampleOnce() {
    long rssKb = 0;
    long vmKb = 0;
    try {
      for (String line : Files.readAllLines(PROC_STATUS)) {
        if (line.startsWith("VmRSS:")) {
          rssKb = parseKb(line);
        } else if (line.startsWith("VmSize:")) {
          vmKb = parseKb(line);
        }
      }
    } catch (Exception e) {
      return;
    }
    samples.add(new Sample(System.currentTimeMillis(), rssKb * 1024, vmKb * 1024));
  }

  private static long parseKb(String line) {
    return Long.parseLong(line.trim().split("\\s+")[1]);
  }
}
