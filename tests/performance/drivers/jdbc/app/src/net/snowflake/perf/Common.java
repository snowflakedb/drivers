package net.snowflake.perf;

import com.sun.management.OperatingSystemMXBean;
import java.lang.management.ManagementFactory;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;

final class Common {

  private static final boolean IS_LINUX =
      System.getProperty("os.name", "").toLowerCase().contains("linux");
  private static final Path PROC_STATUS = Paths.get("/proc/self/status");

  private Common() {}

  /** Peak RSS in MB from {@code /proc/self/status} VmHWM (python parity); 0 off Linux. */
  static double peakRssMb() {
    return IS_LINUX ? readStatusValueKb("VmHWM:") / 1024.0 : 0.0;
  }

  /** Process CPU seconds (user+system, all threads); callers take a delta around the work. */
  static double processCpuSeconds() {
    java.lang.management.OperatingSystemMXBean bean = ManagementFactory.getOperatingSystemMXBean();
    if (bean instanceof OperatingSystemMXBean) {
      long cpuNanos = ((OperatingSystemMXBean) bean).getProcessCpuTime();
      if (cpuNanos >= 0) {
        return cpuNanos / 1_000_000_000.0;
      }
    }
    return 0.0;
  }

  private static long readStatusValueKb(String label) {
    try {
      for (String line : Files.readAllLines(PROC_STATUS)) {
        if (line.startsWith(label)) {
          return Long.parseLong(line.trim().split("\\s+")[1]);
        }
      }
    } catch (Exception e) {
      // unreadable → 0
    }
    return 0L;
  }
}
