package net.snowflake.client.internal.unicore;

import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;

/**
 * Loads the {@code libjdbc_bridge} native library for {@link JNICoreTransport}.
 *
 * <p>Resolution order:
 *
 * <ol>
 *   <li>{@code CORE_PATH} env var — explicit absolute path (back-compat for dev/CI setups).
 *   <li>{@code jdbc.library.path} system property — explicit absolute path.
 *   <li>JAR resource — extract the bundled native lib to a temp file and load from there.
 * </ol>
 *
 * <p>The JAR-resource path is the default for end users: it works in sandboxed JVMs (e.g. UDFs,
 * BucketFS, Lambda) where host env vars and host filesystem layouts aren't predictable.
 *
 * <p>The load runs once, in this class's static initializer. {@link JNICoreTransport}'s constructor
 * calls {@link #init()} to trigger that initialization, ensuring the native lib is in place before
 * any JNI call.
 */
final class NativeLibraryLoader {

  // Bootstrap log must not round-trip through core (native lib is still loading).
  private static final SFLogger logger =
      SFLoggerFactory.getDeliveryLogger(NativeLibraryLoader.class.getName());

  /**
   * Root resource dir for the native libs. The fat JAR carries one lib per platform under an {@code
   * <os>-<arch>} subdir; each {@code snowflake-jdbc-native} artifact carries one. {@link #load()}
   * picks the running platform's at startup.
   */
  private static final String NATIVE_RESOURCE_DIR =
      "/net/snowflake/client/internal/unicore/native/";

  static {
    load();
    logger.info("JDBC driver starting v{}", SnowflakeDriver.getDriverVersion());
  }

  private NativeLibraryLoader() {}

  /** No-op; calling this forces class initialization, which runs the static block above. */
  static void init() {}

  private static void load() {
    String corePath = System.getenv("CORE_PATH");
    if (corePath != null && !corePath.isEmpty()) {
      System.load(corePath);
      return;
    }

    String libraryPath = System.getProperty("jdbc.library.path");
    if (libraryPath != null && !libraryPath.isEmpty()) {
      System.load(libraryPath);
      return;
    }

    Path extracted;
    try {
      extracted = extractNativeLibFromResource();
    } catch (IOException e) {
      throw new RuntimeException(
          "No bundled native library found in JAR resources. The classifier-less "
              + "'net.snowflake:snowflake-jdbc-native' jar is native-free by design and is not runnable "
              + "on its own. For a runnable driver, depend on the platform classifier jar "
              + "'net.snowflake:snowflake-jdbc-native:<version>:<classifier>' (e.g. classifier "
              + "osx-aarch_64; let os-maven-plugin / osdetector-gradle-plugin fill in "
              + "${os.detected.classifier}), or the self-contained "
              + "'net.snowflake:snowflake-jdbc-native-all' jar. Advanced setups can instead point "
              + "CORE_PATH or the jdbc.library.path system property at a local native lib. See "
              + "https://github.com/snowflakedb/drivers/tree/main/jdbc",
          e);
    }
    System.load(extracted.toAbsolutePath().toString());
  }

  /** Copy the bundled native lib resource into a temp file the JVM can {@code System.load}. */
  private static Path extractNativeLibFromResource() throws IOException {
    String libFileName = nativeLibFileName();
    String resourcePath = NATIVE_RESOURCE_DIR + osArchDir() + "/" + libFileName;
    try (InputStream in = NativeLibraryLoader.class.getResourceAsStream(resourcePath)) {
      if (in == null) {
        throw new IOException(
            "Native library not found in JAR at "
                + resourcePath
                + ". This JAR carries no native lib for this platform ("
                + System.getProperty("os.name")
                + "/"
                + System.getProperty("os.arch")
                + ") — expected for the native-free 'snowflake-jdbc-native' jar, or for a classifier "
                + "jar built for a different platform.");
      }
      // Use a per-version subdir so concurrent JVMs don't clobber each other and the OS
      // doesn't refuse to load a file that's been overwritten while still mapped.
      Path tmpDir =
          Files.createTempDirectory("snowflake-jdbc-native-" + SnowflakeDriver.getDriverVersion());
      tmpDir.toFile().deleteOnExit();
      Path tmpLib = tmpDir.resolve(libFileName);
      Files.copy(in, tmpLib, StandardCopyOption.REPLACE_EXISTING);
      tmpLib.toFile().deleteOnExit();
      return tmpLib;
    }
  }

  /** Native lib filename cargo produces on the current OS. Keyed off {@link #osToken()}. */
  private static String nativeLibFileName() {
    switch (osToken()) {
      case "darwin":
        return "libjdbc_bridge.dylib";
      case "windows":
        return "jdbc_bridge.dll";
      default:
        // Linux and other POSIX systems
        return "libjdbc_bridge.so";
    }
  }

  /**
   * The resource subdir for the running platform: {@code <os>-<arch>-<libc>} on Linux (glibc and
   * musl natives aren't interchangeable, so each jar carries both variants and we pick here), just
   * {@code <os>-<arch>} on macOS/Windows.
   *
   * <p>SYNC CONTRACT: {@link #osToken()}/{@link #archToken()}/{@link #libcToken()} must match the
   * {@code host*Token} derivations in {@code build.gradle} and the matrix in {@code
   * _build-jdbc-fatjar.yml}, which name the jar's subdirs.
   */
  private static String osArchDir() {
    String osArch = osToken() + "-" + archToken();
    // Only Linux ships a per-libc native; mac/windows subdirs carry no libc segment.
    if ("linux".equals(osToken())) {
      return osArch + "-" + libcToken();
    }
    return osArch;
  }

  /**
   * The C library flavor of the running host: {@code "musl"} or {@code "gnu"}. Only consulted on
   * Linux (see {@link #osArchDir()}). The {@code jdbc.libc} system property overrides {@link
   * LibcDetector#detect()} for locked-down JVMs; defaults to {@code "gnu"} when detection is
   * inconclusive (glibc is the common case and the override is the escape hatch).
   */
  private static String libcToken() {
    String override = System.getProperty("jdbc.libc");
    if (override != null && !override.isEmpty()) {
      return override.toLowerCase();
    }
    return "musl".equals(LibcDetector.detect()) ? "musl" : "gnu";
  }

  private static String osToken() {
    String osName = System.getProperty("os.name", "").toLowerCase();
    if (osName.contains("mac") || osName.contains("darwin")) {
      return "darwin";
    }
    if (osName.contains("windows")) {
      return "windows";
    }
    return "linux";
  }

  private static String archToken() {
    String osArch = System.getProperty("os.arch", "").toLowerCase();
    if (osArch.equals("aarch64") || osArch.equals("arm64")) {
      return "aarch64";
    }
    if (osArch.equals("amd64") || osArch.equals("x86_64")) {
      return "x86_64";
    }
    // Fall through with the raw value so the "not found" error names the actual arch.
    return osArch;
  }
}
