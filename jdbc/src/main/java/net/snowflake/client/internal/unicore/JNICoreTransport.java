package net.snowflake.client.internal.unicore;

import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;

public class JNICoreTransport implements CoreTransport {
  private static final SFLogger logger = SFLoggerFactory.getLogger(JNICoreTransport.class);

  /** Resource directory inside the JAR where {@code copyNativeLib} (build.gradle) places the lib. */
  private static final String NATIVE_RESOURCE_DIR =
      "/net/snowflake/client/internal/unicore/native/";

  static {
    loadNativeLibrary();
    logger.info("JDBC driver starting v{}", SnowflakeDriver.getDriverVersion());
  }

  /**
   * Load the {@code libjdbc_bridge} native library.
   *
   * <p>Resolution order:
   *
   * <ol>
   *   <li>{@code CORE_PATH} env var — explicit absolute path (back-compat for dev/CI setups).
   *   <li>{@code jdbc.library.path} system property — explicit absolute path.
   *   <li>JAR resource — extract the bundled native lib to a temp file and load from there.
   * </ol>
   *
   * <p>The JAR-resource path is the default for end users: it works in sandboxed JVMs
   * (e.g. UDFs, BucketFS, Lambda) where host env vars and host filesystem layouts aren't
   * predictable.
   */
  private static void loadNativeLibrary() {
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
          "Failed to extract bundled native library from JAR resources. Either ensure the lib is "
              + "bundled in the JAR (built via Gradle copyNativeLib) or set CORE_PATH / "
              + "jdbc.library.path explicitly.",
          e);
    }
    System.load(extracted.toAbsolutePath().toString());
  }

  /** Copy the bundled native lib resource into a temp file the JVM can {@code System.load}. */
  private static Path extractNativeLibFromResource() throws IOException {
    String libFileName = nativeLibFileName();
    String resourcePath = NATIVE_RESOURCE_DIR + libFileName;
    try (InputStream in = JNICoreTransport.class.getResourceAsStream(resourcePath)) {
      if (in == null) {
        throw new IOException(
            "Native library not found in JAR at " + resourcePath + ". The driver JAR was built "
                + "without the platform-specific native lib bundled.");
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

  /**
   * Resolve the native lib filename for the current OS. Architecture isn't currently encoded in
   * the resource path because cargo only produces one binary per OS in {@code target/release/};
   * if we ship multi-arch builds in the future, extend this to include {@code os.arch}.
   */
  private static String nativeLibFileName() {
    String osName = System.getProperty("os.name", "").toLowerCase();
    if (osName.contains("mac") || osName.contains("darwin")) {
      return "libjdbc_bridge.dylib";
    }
    if (osName.contains("windows")) {
      return "jdbc_bridge.dll";
    }
    // Linux and other POSIX systems
    return "libjdbc_bridge.so";
  }

  @Override
  public TransportResponse handleMessage(String serviceName, String methodName, byte[] requestBytes)
      throws TransportException {
    logger.trace(
        "JNI transport request: service={}, method={}, requestBytes={}",
        serviceName,
        methodName,
        requestBytes == null ? -1 : requestBytes.length);
    TransportResponse response = nativeHandleMessage(serviceName, methodName, requestBytes);
    if (response == null) {
      logger.warn(
          "JNI transport returned null response: service={}, method={}", serviceName, methodName);
      throw new TransportException("Empty transport response");
    }
    byte[] responseBytes = response.getResponseBytes();
    logger.trace(
        "JNI transport response: service={}, method={}, code={}, responseBytes={}",
        serviceName,
        methodName,
        response.getCode(),
        responseBytes == null ? -1 : responseBytes.length);
    return response;
  }

  private static native TransportResponse nativeHandleMessage(
      String serviceName, String methodName, byte[] requestBytes);
}
