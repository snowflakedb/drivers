package net.snowflake.client.internal.unicore;

import java.io.IOException;
import java.io.InputStream;
import java.net.URISyntaxException;
import java.net.URL;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardCopyOption;
import java.security.CodeSource;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Enumeration;
import java.util.List;
import java.util.Locale;
import java.util.jar.JarEntry;
import java.util.jar.JarFile;
import java.util.jar.Manifest;

/** Keeps candidate native selection within the JAR that defined the loader, not the classpath. */
final class CandidateNativeLibrary implements AutoCloseable {
  private static final String MARKER = "Snowflake-JDBC-FIPS-Candidate";
  private static final String NATIVE_DIR = "net/snowflake/client/internal/unicore/native/";

  private final JarFile jar;

  private CandidateNativeLibrary(JarFile jar) {
    this.jar = jar;
  }

  static CandidateNativeLibrary fromCodeSource(CodeSource codeSource) throws IOException {
    if (codeSource == null || codeSource.getLocation() == null) {
      throw new IOException("Cannot inspect JDBC driver defining archive: missing code source");
    }
    URL location = codeSource.getLocation();
    if (!"file".equals(location.getProtocol())) {
      throw new IOException("Cannot inspect JDBC driver defining archive: " + location);
    }
    Path path;
    try {
      path = Paths.get(location.toURI());
    } catch (URISyntaxException e) {
      throw new IOException("Invalid JDBC driver code source: " + location, e);
    }
    if (Files.isDirectory(path)) {
      return null;
    }
    if (!Files.isRegularFile(path)) {
      throw new IOException("JDBC driver defining archive is not a regular file: " + path);
    }
    // Inspect the marker and extract the native from the same JAR handle.
    JarFile jar = new JarFile(path.toFile());
    boolean candidate = false;
    try {
      if (!hasCandidateMarker(jar)) {
        return null;
      }
      candidate = true;
      return new CandidateNativeLibrary(jar);
    } finally {
      if (!candidate) {
        jar.close();
      }
    }
  }

  Path extract(
      String corePath, String libraryPath, String osName, String platformDir, String nativeFile)
      throws IOException {
    if (corePath != null && !corePath.isEmpty()) {
      throw new IllegalStateException("FIPS-TLS candidate JAR refuses CORE_PATH native override");
    }
    if (libraryPath != null && !libraryPath.isEmpty()) {
      throw new IllegalStateException(
          "FIPS-TLS candidate JAR refuses jdbc.library.path native override");
    }
    if (!supportedPlatform(osName, platformDir, nativeFile)) {
      throw new IllegalStateException(
          "FIPS-TLS candidate JAR has unsupported host platform: " + osName + "/" + platformDir);
    }

    String resourcePath = NATIVE_DIR + platformDir + "/" + nativeFile;
    List<String> nativeResources = new ArrayList<>();
    Enumeration<JarEntry> entries = jar.entries();
    while (entries.hasMoreElements()) {
      JarEntry entry = entries.nextElement();
      String name = entry.getName();
      if (!entry.isDirectory()
          && (name.startsWith(NATIVE_DIR)
              || name.endsWith("libjdbc_bridge.so")
              || name.endsWith("libjdbc_bridge.dylib")
              || name.endsWith("jdbc_bridge.dll"))) {
        nativeResources.add(name);
      }
    }
    if (!nativeResources.equals(Collections.singletonList(resourcePath))) {
      throw new IOException(
          "FIPS-TLS candidate JAR expected exactly "
              + resourcePath
              + " but contains native resources "
              + nativeResources);
    }
    try (InputStream input = jar.getInputStream(jar.getJarEntry(resourcePath))) {
      Path tmpDir = Files.createTempDirectory("snowflake-jdbc-fips-candidate-");
      tmpDir.toFile().deleteOnExit();
      Path tmpLib = tmpDir.resolve(nativeFile);
      tmpLib.toFile().deleteOnExit();
      Files.copy(input, tmpLib, StandardCopyOption.REPLACE_EXISTING);
      return tmpLib;
    }
  }

  @Override
  public void close() throws IOException {
    jar.close();
  }

  private static boolean hasCandidateMarker(JarFile jar) throws IOException {
    Manifest manifest = jar.getManifest();
    if (manifest == null) {
      return false;
    }
    String value = manifest.getMainAttributes().getValue(MARKER);
    if (value == null) {
      return false;
    }
    if (!"true".equals(value)) {
      throw new IOException("Invalid FIPS-TLS candidate marker in defining JAR");
    }
    return true;
  }

  private static boolean supportedPlatform(String osName, String platformDir, String nativeFile) {
    String os = osName.toLowerCase(Locale.ROOT);
    if (os.contains("linux")) {
      return nativeFile.equals("libjdbc_bridge.so")
          && platformDir.matches("linux-(x86_64|aarch64)-(gnu|musl)");
    }
    if (os.contains("mac") || os.contains("darwin")) {
      return nativeFile.equals("libjdbc_bridge.dylib")
          && platformDir.matches("darwin-(x86_64|aarch64)");
    }
    if (os.contains("windows")) {
      return nativeFile.equals("jdbc_bridge.dll") && platformDir.equals("windows-x86_64");
    }
    return false;
  }
}
