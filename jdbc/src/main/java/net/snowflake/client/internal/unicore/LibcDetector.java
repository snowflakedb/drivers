package net.snowflake.client.internal.unicore;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.concurrent.TimeUnit;
import java.util.regex.Pattern;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;

/**
 * Detects the host's C library family so {@link NativeLibraryLoader} can pick the matching Linux
 * native ({@code gnu} vs {@code musl}); a {@code .so} linked against glibc won't load against musl
 * and vice versa. Detection mirrors snowflake-jdbc's {@code LibcDetails}; glibc maps to this
 * loader's {@code "gnu"} subdir token.
 *
 * <p>Kept as its own class — with no static initializer and no native dependency — so the parsing
 * logic is unit-testable without triggering {@link NativeLibraryLoader}'s native-library load.
 */
final class LibcDetector {

  private static final SFLogger logger =
      SFLoggerFactory.getDeliveryLogger(LibcDetector.class.getName());

  private static final String DEFAULT_LDD_PATH = "/usr/bin/ldd";
  // Word boundaries guard against false positives like "muslib" or "muscle".
  private static final Pattern RE_MUSL_MARKER = Pattern.compile("\\bmusl\\b");
  private static final Pattern RE_GLIBC_NAME_MARKER = Pattern.compile("\\bGNU C Library\\b");
  private static final Pattern RE_GLIBC_GETCONF_MARKER = Pattern.compile("\\bglibc\\b");
  private static final long LIBC_EXEC_TIMEOUT_MS = 200;

  private LibcDetector() {}

  /**
   * Detects the host's libc family as a loader subdir token ({@code "musl"} / {@code "gnu"}), or
   * {@code null} when inconclusive. Reads {@code /usr/bin/ldd} (a shell script on glibc, a symlink
   * to the musl loader on Alpine), then falls back to {@code getconf GNU_LIBC_VERSION} / {@code ldd
   * --version}.
   */
  static String detect() {
    String fromFile = parseLddContent(readFileUtf8(Paths.get(DEFAULT_LDD_PATH)));
    if (fromFile != null) {
      return fromFile;
    }
    return parseCommandOutput(runLibcVersionCommands());
  }

  static String parseLddContent(String content) {
    if (content == null || content.isEmpty()) {
      return null;
    }
    if (RE_MUSL_MARKER.matcher(content).find()) {
      return "musl";
    }
    if (RE_GLIBC_NAME_MARKER.matcher(content).find()) {
      return "gnu";
    }
    return null;
  }

  static String parseCommandOutput(String output) {
    if (output == null || output.isEmpty()) {
      return null;
    }
    String[] lines = output.split("\\R+");
    String getconfLine = lines.length > 0 ? lines[0] : null;
    String lddLine1 = lines.length > 1 ? lines[1] : null;
    if (getconfLine != null && RE_GLIBC_GETCONF_MARKER.matcher(getconfLine).find()) {
      return "gnu";
    }
    if (lddLine1 != null && RE_MUSL_MARKER.matcher(lddLine1).find()) {
      return "musl";
    }
    return null;
  }

  private static String readFileUtf8(Path path) {
    try {
      return new String(Files.readAllBytes(path), StandardCharsets.UTF_8);
    } catch (IOException | RuntimeException e) {
      logger.debug("Failed to read libc details from {}: {}", path, e.getMessage());
      return null;
    }
  }

  private static String runLibcVersionCommands() {
    ProcessBuilder pb =
        new ProcessBuilder(
            "/bin/sh", "-c", "getconf GNU_LIBC_VERSION 2>&1 || true; ldd --version 2>&1 || true");
    pb.redirectErrorStream(true);
    Process process = null;
    try {
      process = pb.start();
      String output = readStreamUtf8(process.getInputStream());
      if (!process.waitFor(LIBC_EXEC_TIMEOUT_MS, TimeUnit.MILLISECONDS)) {
        process.destroyForcibly();
        logger.debug("Libc version command timed out after {}ms", LIBC_EXEC_TIMEOUT_MS);
        return null;
      }
      return output;
    } catch (IOException e) {
      logger.debug("Failed to run libc version command: {}", e.getMessage());
      return null;
    } catch (InterruptedException e) {
      Thread.currentThread().interrupt();
      logger.debug("Interrupted while running libc version command: {}", e.getMessage());
      return null;
    } catch (Exception e) {
      logger.debug("Unexpected error running libc version command: {}", e.getMessage());
      return null;
    } finally {
      if (process != null && process.isAlive()) {
        process.destroyForcibly();
      }
    }
  }

  private static String readStreamUtf8(InputStream in) throws IOException {
    ByteArrayOutputStream buf = new ByteArrayOutputStream();
    byte[] chunk = new byte[1024];
    int n;
    while ((n = in.read(chunk)) != -1) {
      buf.write(chunk, 0, n);
    }
    return new String(buf.toByteArray(), StandardCharsets.UTF_8);
  }
}
