package net.snowflake.client.internal.unicore;

import java.io.IOException;
import java.net.URL;
import java.nio.file.Path;
import java.nio.file.Paths;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
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
 *   <li>JAR resource — extract the bundled native lib, plus any companion libraries listed in the
 *       platform dir's {@code companions.txt}, to a temp dir and load from there. See {@link
 *       BundledNativeLibrary}.
 * </ol>
 *
 * <p>{@code CORE_PATH} and {@code jdbc.library.path} name a single file and load no companions. A
 * FIPS build on macOS or Windows loaded that way needs its AWS-LC companion libraries resolvable by
 * the OS loader on its own (for example on {@code PATH} on Windows).
 *
 * <p>The JAR-resource path is the default for end users: it works in sandboxed JVMs (e.g. UDFs,
 * BucketFS, Lambda) where host env vars and host filesystem layouts aren't predictable.
 *
 * <p>{@code SnowflakeDriver} calls {@link #loadIfPresent()} while the driver class initializes so
 * extract/{@code System.load} is not on the first {@code connect()} path. {@link #init()} remains
 * the safety net for first JNI use. Any runtime or linkage failure is wrapped, logged at error,
 * cached, and rethrown as {@code SQLException} from {@code connect()} / JNI. It does not escape
 * {@code SnowflakeDriver}'s initializer, which would poison the class as {@code
 * NoClassDefFoundError} and make DriverManager SPI report no suitable driver.
 */
public final class NativeLibraryLoader {

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

  private static final Object LOCK = new Object();

  private static volatile boolean loaded;
  private static volatile SFSQLException loadFailure;

  /** Retained so the extract-directory lock is not garbage-collected. */
  @SuppressWarnings("unused")
  private static NativeLibraryTempDirs.ExtractDir extractDir;

  private NativeLibraryLoader() {}

  /**
   * Loads the native library during driver class initialization when a bundled lib or an explicit
   * path is available, so {@code connect()} does not pay extract/{@code System.load} on the first
   * user call. A failure is cached and logged by {@link #init()}; it is not rethrown here.
   */
  public static void loadIfPresent() {
    if (explicitLibraryPath() == null && !bundledNativePresent()) {
      return;
    }
    try {
      init();
    } catch (SFSQLException e) {
      // Already cached and logged by init().
    }
  }

  /**
   * Loads the native library if it is not already loaded. Subsequent calls after a failure rethrow
   * the same {@link SFSQLException}.
   */
  static void init() {
    if (loaded) {
      return;
    }
    SFSQLException previous = loadFailure;
    if (previous != null) {
      throw previous;
    }
    synchronized (LOCK) {
      if (loaded) {
        return;
      }
      if (loadFailure != null) {
        throw loadFailure;
      }
      try {
        load();
        loaded = true;
        logger.info("JDBC driver starting v{}", SnowflakeDriver.getDriverVersion());
      } catch (SFSQLException e) {
        loadFailure = e;
        logger.error("Failed to load the JDBC native library", e);
        throw e;
      } catch (RuntimeException | LinkageError e) {
        SFSQLException wrapped = nativeLoadFailure(loadFailedMessage(null, e.toString()), e);
        loadFailure = wrapped;
        logger.error("Failed to load the JDBC native library", wrapped);
        throw wrapped;
      }
    }
  }

  private static void load() {
    String explicit = explicitLibraryPath();
    if (explicit != null) {
      loadPath(explicit);
      return;
    }

    BundledNativeLibrary bundle = readBundledLibrary();
    loadExtracted(
        extractTo(
            bundle,
            Paths.get(System.getProperty("java.io.tmpdir")),
            SnowflakeDriver.getDriverVersion()),
        bundle);
  }

  /**
   * Loads every file of {@code bundle} from {@code extracted}, companions first. A load failure
   * closes the extract lock so a cached error does not hold the file descriptor until the JVM
   * exits.
   */
  static void loadExtracted(
      NativeLibraryTempDirs.ExtractDir extracted, BundledNativeLibrary bundle) {
    try {
      for (String name : bundle.loadOrder()) {
        loadPath(extracted.dir.resolve(name).toAbsolutePath().toString());
      }
    } catch (RuntimeException | LinkageError e) {
      closeExtract(extracted, e);
      throw e;
    }
    extractDir = extracted;
    NativeLibraryTempDirs.deleteStaleExtractsInBackground(extracted.dir);
  }

  private static String explicitLibraryPath() {
    String corePath = System.getenv("CORE_PATH");
    if (corePath != null && !corePath.isEmpty()) {
      return corePath;
    }
    String libraryPath = System.getProperty("jdbc.library.path");
    if (libraryPath != null && !libraryPath.isEmpty()) {
      return libraryPath;
    }
    return null;
  }

  private static String bundledResourceDir() {
    return NATIVE_RESOURCE_DIR + osArchDir() + "/";
  }

  private static String bundledResourcePath() {
    return bundledResourceDir() + nativeLibFileName();
  }

  static boolean bundledNativePresent() {
    return bundledNativeUrl() != null;
  }

  static URL bundledNativeUrl() {
    return NativeLibraryLoader.class.getResource(bundledResourcePath());
  }

  private static void loadPath(String path) {
    try {
      System.load(path);
    } catch (UnsatisfiedLinkError e) {
      throw nativeLoadFailure(loadFailedMessage(path, e.getMessage()), e);
    }
  }

  private static BundledNativeLibrary readBundledLibrary() {
    String resourcePath = bundledResourcePath();
    if (!bundledNativePresent()) {
      String message =
          onlyGlibcNativeBundled()
              ? glibcOnlyMessage(resourcePath)
              : missingNativeMessage(
                  resourcePath,
                  System.getProperty("os.name"),
                  System.getProperty("os.arch"),
                  mavenClassifier(osToken(), archToken()));
      throw nativeLoadFailure(message, null);
    }
    try {
      return BundledNativeLibrary.read(
          bundledResourceDir(),
          nativeLibFileName(),
          NativeLibraryLoader.class::getResourceAsStream);
    } catch (IOException e) {
      throw nativeLoadFailure(manifestFailedMessage(resourcePath, e.getMessage()), e);
    }
  }

  /**
   * Copies {@code bundle} into a new extract directory under {@code tmpRoot}. The files are
   * registered for {@code deleteOnExit} after the directory and its {@code .lock}, so a clean exit
   * deletes the files, then {@code .lock}, then the empty directory.
   */
  static NativeLibraryTempDirs.ExtractDir extractTo(
      BundledNativeLibrary bundle, Path tmpRoot, String version) {
    NativeLibraryTempDirs.ExtractDir extracted;
    try {
      extracted = NativeLibraryTempDirs.create(tmpRoot, version);
    } catch (IOException e) {
      throw nativeLoadFailure(extractFailedMessage(tmpRoot.toString(), e.getMessage()), e);
    }
    try {
      bundle.copyInto(extracted.dir);
    } catch (IOException e) {
      SFSQLException failure =
          nativeLoadFailure(extractFailedMessage(extracted.dir.toString(), e.getMessage()), e);
      closeExtract(extracted, failure);
      throw failure;
    }
    return extracted;
  }

  private static void closeExtract(NativeLibraryTempDirs.ExtractDir extracted, Throwable failure) {
    try {
      extracted.close();
    } catch (IOException closeFailure) {
      failure.addSuppressed(closeFailure);
    }
  }

  static String mavenClassifier(String osToken, String archToken) {
    String mavenArch = "aarch64".equals(archToken) ? "aarch_64" : archToken;
    if ("darwin".equals(osToken)) {
      return "osx-" + mavenArch;
    }
    if ("windows".equals(osToken)) {
      return "windows-" + mavenArch;
    }
    return "linux-" + mavenArch;
  }

  static String missingNativeMessage(
      String resourcePath, String osName, String osArch, String classifier) {
    return "Native library not found in JAR at "
        + resourcePath
        + ". This JAR carries no native library for this platform ("
        + osName
        + "/"
        + osArch
        + "). Use classifier "
        + classifier
        + " (net.snowflake:snowflake-jdbc-native:<version>:"
        + classifier
        + " or net.snowflake:snowflake-jdbc-native-fips:<version>:"
        + classifier
        + ") or the self-contained net.snowflake:snowflake-jdbc-native-all / "
        + "net.snowflake:snowflake-jdbc-native-all-fips jar.";
  }

  /**
   * True when this process is a musl Linux host and {@code gnuNativePresent} is the glibc native
   * for the same arch. Used when the musl resource is missing so the error can name a glibc-only
   * jar instead of a missing platform.
   */
  static boolean onlyGlibcNativeBundled(
      String osToken, String libcToken, boolean gnuNativePresent) {
    return "linux".equals(osToken) && "musl".equals(libcToken) && gnuNativePresent;
  }

  private static boolean onlyGlibcNativeBundled() {
    String glibcPath =
        NATIVE_RESOURCE_DIR + osToken() + "-" + archToken() + "-gnu/" + nativeLibFileName();
    return onlyGlibcNativeBundled(
        osToken(), libcToken(), NativeLibraryLoader.class.getResource(glibcPath) != null);
  }

  static String glibcOnlyMessage(String resourcePath) {
    return "Native library not found in JAR at "
        + resourcePath
        + ". This JAR carries only the glibc native for this platform. Run on a glibc-based"
        + " image, or set -Djdbc.libc=gnu if this host is glibc and was detected as musl.";
  }

  static String manifestFailedMessage(String resourcePath, String detail) {
    return "Failed to read the companion library list next to "
        + resourcePath
        + (detail != null ? ": " + detail : ".")
        + " The JAR is damaged or was repackaged without it; use the published driver jar.";
  }

  static String extractFailedMessage(String location, String detail) {
    return "Failed to extract the bundled native library"
        + (location != null ? " under " + location : "")
        + (detail != null ? ": " + detail : ".")
        + " The directory must be writable and have about 30 MB free. Override with"
        + " -Djava.io.tmpdir.";
  }

  static String loadFailedMessage(String path, String detail) {
    return "Failed to load the native library"
        + (path != null ? " from " + path : "")
        + (detail != null ? ": " + detail : ".")
        + " If the path is under /tmp, it must allow exec (noexec mounts fail with 'failed to"
        + " map segment'). Linux gnu natives need glibc 2.28+. Alpine/musl images need libgcc"
        + " (apk add --no-cache libgcc). Override the extract directory with -Djava.io.tmpdir.";
  }

  private static SFSQLException nativeLoadFailure(String message, Throwable cause) {
    return new SFSQLException(ErrorCode.NATIVE_LIBRARY_LOAD_FAILED, message, cause);
  }

  /** Native lib filename cargo produces on the current OS. Keyed off {@link #osToken()}. */
  static String nativeLibFileName() {
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
