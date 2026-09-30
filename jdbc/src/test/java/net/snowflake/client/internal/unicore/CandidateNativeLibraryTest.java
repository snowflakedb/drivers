package net.snowflake.client.internal.unicore;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.URL;
import java.net.URLClassLoader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.CodeSource;
import java.security.cert.Certificate;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.jar.Attributes;
import java.util.jar.JarEntry;
import java.util.jar.JarOutputStream;
import java.util.jar.Manifest;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

/** Exercises the defining-JAR boundary without loading a native library or requiring a FIPS build. */
public class CandidateNativeLibraryTest {
  private static final String NATIVE_PATH =
      "net/snowflake/client/internal/unicore/native/linux-x86_64-gnu/libjdbc_bridge.so";
  private static final String OTHER_NATIVE_PATH =
      "net/snowflake/client/internal/unicore/native/darwin-aarch64/libjdbc_bridge.dylib";

  @TempDir Path tempDir;
  private final List<CandidateNativeLibrary> openedJars = new ArrayList<>();

  @AfterEach
  public void closeOpenedJars() throws IOException {
    for (CandidateNativeLibrary candidate : openedJars) {
      candidate.close();
    }
  }

  @Test
  public void shouldRejectOverrideForCandidateArchiveWithoutJarSuffix() throws Exception {
    Path path = jar("renamed.zip", true, Collections.singletonMap(NATIVE_PATH, "candidate-native"));
    CandidateNativeLibrary candidate = openedCandidate(codeSource(path));

    IllegalStateException error =
        assertThrows(
            IllegalStateException.class,
            () -> candidate.extract("/tmp/ordinary.so", null, "Linux", "linux-x86_64-gnu", "libjdbc_bridge.so"));

    assertTrue(error.getMessage().contains("CORE_PATH"));
  }

  @Test
  public void shouldFailWhenDefiningArchiveCannotBeInspected() {
    IOException error =
        assertThrows(
            IOException.class,
            () -> CandidateNativeLibrary.fromCodeSource(null, getClass().getClassLoader()));
    assertTrue(error.getMessage().contains("missing code source"));
  }

  @Test
  public void shouldRejectCorePathBeforeExtractingCandidateNative() throws Exception {
    CandidateNativeLibrary candidate = candidateJarWith(NATIVE_PATH, "native");

    IllegalStateException error =
        assertThrows(
            IllegalStateException.class,
            () -> candidate.extract("/tmp/ordinary-bridge.so", null, "Linux", "linux-x86_64-gnu", "libjdbc_bridge.so"));

    assertTrue(error.getMessage().contains("CORE_PATH"));
  }

  @Test
  public void shouldRejectLibraryPathBeforeExtractingCandidateNative() throws Exception {
    CandidateNativeLibrary candidate = candidateJarWith(NATIVE_PATH, "native");

    IllegalStateException error =
        assertThrows(
            IllegalStateException.class,
            () -> candidate.extract(null, "/tmp/ordinary-bridge.so", "Linux", "linux-x86_64-gnu", "libjdbc_bridge.so"));

    assertTrue(error.getMessage().contains("jdbc.library.path"));
  }

  @Test
  public void shouldExtractOnlyMatchingNativeFromDefiningCandidateJar() throws Exception {
    byte[] expected = "candidate-specific-native".getBytes(StandardCharsets.UTF_8);
    CandidateNativeLibrary candidate = candidateJarWith(NATIVE_PATH, "candidate-specific-native");

    Path extracted = candidate.extract("", "", "Linux", "linux-x86_64-gnu", "libjdbc_bridge.so");

    try {
      assertArrayEquals(expected, Files.readAllBytes(extracted));
    } finally {
      Files.deleteIfExists(extracted);
      Files.deleteIfExists(extracted.getParent());
    }
  }

  @Test
  public void shouldRejectOrdinaryDefiningJarWhenCandidateResourceSharesClassloader()
      throws Exception {
    Path ordinary = ordinaryLoaderJar();
    Path candidate =
        jar(
            "candidate-resource.jar",
            true,
            Collections.singletonMap(
                "META-INF/snowflake/jdbc/fips-tls-candidate.marker", "FIPS-TLS candidate"));

    try (URLClassLoader classpath = isolatedLoader(ordinary, candidate)) {
      Class<?> loader =
          Class.forName(NativeLibraryLoader.class.getName(), false, classpath);
      assertEquals(classpath, loader.getClassLoader());
      ExceptionInInitializerError error =
          assertThrows(
              ExceptionInInitializerError.class,
              () -> Class.forName(loader.getName(), true, classpath));
      assertTrue(error.getCause() instanceof IllegalStateException);
      assertTrue(error.getCause().getMessage().contains("FIPS-TLS candidate"));
    }
  }

  @Test
  public void shouldAllowUnmarkedOrdinaryJarWithoutCandidateResource() throws Exception {
    Path ordinary = ordinaryLoaderJar();
    try (URLClassLoader classpath = isolatedLoader(ordinary)) {
      Class<?> loader = Class.forName(NativeLibraryLoader.class.getName(), false, classpath);
      assertEquals(classpath, loader.getClassLoader());
      assertNull(
          CandidateNativeLibrary.fromCodeSource(
              loader.getProtectionDomain().getCodeSource(), loader.getClassLoader()));
    }
  }

  @Test
  public void shouldIgnoreCandidateMarkerInAnotherClasspathJar() throws Exception {
    Path ordinary = jar("ordinary.jar", false, Collections.emptyMap());
    Map<String, String> contents = new LinkedHashMap<>();
    contents.put(NATIVE_PATH, "foreign-native");
    contents.put("META-INF/snowflake/jdbc/fips-tls-candidate.marker", "FIPS-TLS candidate");
    Path candidate = jar("candidate.jar", true, contents);
    ClassLoader oldContextLoader = Thread.currentThread().getContextClassLoader();
    try (URLClassLoader classpath = new URLClassLoader(new java.net.URL[] {candidate.toUri().toURL()})) {
      Thread.currentThread().setContextClassLoader(classpath);
      assertNull(
          CandidateNativeLibrary.fromCodeSource(
              codeSource(ordinary), CandidateNativeLibrary.class.getClassLoader()));
    } finally {
      Thread.currentThread().setContextClassLoader(oldContextLoader);
    }
  }

  @Test
  public void shouldRefuseToBorrowNativeFromAnotherClasspathJar() throws Exception {
    Path marked = jar("marked.jar", true, Collections.emptyMap());
    Path foreign = jar("foreign.jar", false, Collections.singletonMap(NATIVE_PATH, "foreign-native"));
    CandidateNativeLibrary candidate = openedCandidate(codeSource(marked));
    ClassLoader oldContextLoader = Thread.currentThread().getContextClassLoader();
    try (URLClassLoader classpath = new URLClassLoader(new java.net.URL[] {foreign.toUri().toURL()})) {
      Thread.currentThread().setContextClassLoader(classpath);
      IOException error =
          assertThrows(
              IOException.class,
              () -> candidate.extract(null, null, "Linux", "linux-x86_64-gnu", "libjdbc_bridge.so"));
      assertTrue(error.getMessage().contains("expected exactly"));
    } finally {
      Thread.currentThread().setContextClassLoader(oldContextLoader);
    }
  }

  @Test
  public void shouldRefuseExtraNativeEvenWhenHostNativeIsPresent() throws Exception {
    Map<String, String> contents = new LinkedHashMap<>();
    contents.put(NATIVE_PATH, "candidate-native");
    contents.put(OTHER_NATIVE_PATH, "ordinary-native");
    Path marked = jar("mixed.jar", true, contents);
    CandidateNativeLibrary candidate = openedCandidate(codeSource(marked));

    IOException error =
        assertThrows(
            IOException.class,
            () -> candidate.extract(null, null, "Linux", "linux-x86_64-gnu", "libjdbc_bridge.so"));

    assertTrue(error.getMessage().contains("expected exactly"));
  }

  @Test
  public void shouldRefuseOrdinaryBridgeHiddenOutsideCandidateResourceRoot() throws Exception {
    Map<String, String> contents = new LinkedHashMap<>();
    contents.put(NATIVE_PATH, "candidate-native");
    contents.put("other/native/libjdbc_bridge.so", "ordinary-native");
    Path marked = jar("mixed-paths.jar", true, contents);
    CandidateNativeLibrary candidate = openedCandidate(codeSource(marked));

    IOException error =
        assertThrows(
            IOException.class,
            () -> candidate.extract(null, null, "Linux", "linux-x86_64-gnu", "libjdbc_bridge.so"));

    assertTrue(error.getMessage().contains("other/native/libjdbc_bridge.so"));
  }

  @Test
  public void shouldRefuseNativeFromWrongPlatform() throws Exception {
    CandidateNativeLibrary candidate = candidateJarWith(OTHER_NATIVE_PATH, "wrong-platform-native");

    IOException error =
        assertThrows(
            IOException.class,
            () -> candidate.extract(null, null, "Linux", "linux-x86_64-gnu", "libjdbc_bridge.so"));

    assertTrue(error.getMessage().contains("expected exactly"));
  }

  @Test
  public void shouldRefuseUnsupportedPlatformBeforeExtracting() throws Exception {
    CandidateNativeLibrary candidate = candidateJarWith(NATIVE_PATH, "candidate-native");

    IllegalStateException error =
        assertThrows(
            IllegalStateException.class,
            () -> candidate.extract(null, null, "FreeBSD", "linux-x86_64-gnu", "libjdbc_bridge.so"));

    assertTrue(error.getMessage().contains("unsupported host platform"));
  }

  @Test
  public void shouldLeaveUnmarkedOrdinaryJarOutsideCandidatePolicy() throws Exception {
    Path ordinary = jar("ordinary.jar", false, Collections.singletonMap(NATIVE_PATH, "ordinary-native"));
    assertNull(
        CandidateNativeLibrary.fromCodeSource(
            codeSource(ordinary), CandidateNativeLibrary.class.getClassLoader()));
  }

  private Path ordinaryLoaderJar() throws IOException {
    // The class bytes must come from the ordinary URL, not the parent test classloader.
    Path path = tempDir.resolve("ordinary-loader.jar");
    Manifest manifest = new Manifest();
    manifest.getMainAttributes().put(Attributes.Name.MANIFEST_VERSION, "1.0");
    try (OutputStream output = Files.newOutputStream(path);
        JarOutputStream jar = new JarOutputStream(output, manifest)) {
      for (Class<?> type : new Class<?>[] {NativeLibraryLoader.class, CandidateNativeLibrary.class}) {
        String resource = type.getName().replace('.', '/') + ".class";
        jar.putNextEntry(new JarEntry(resource));
        try (InputStream in = type.getClassLoader().getResourceAsStream(resource)) {
          if (in == null) {
            throw new IOException("Missing test class bytes: " + resource);
          }
          byte[] buffer = new byte[8192];
          int count;
          while ((count = in.read(buffer)) != -1) {
            jar.write(buffer, 0, count);
          }
        }
        jar.closeEntry();
      }
    }
    return path;
  }

  private static URLClassLoader isolatedLoader(Path... jars) throws IOException {
    URL[] urls = new URL[jars.length];
    for (int index = 0; index < jars.length; index++) {
      urls[index] = jars[index].toUri().toURL();
    }
    return new URLClassLoader(urls, CandidateNativeLibraryTest.class.getClassLoader()) {
      @Override
      protected Class<?> loadClass(String name, boolean resolve) throws ClassNotFoundException {
        if (name.equals(NativeLibraryLoader.class.getName())
            || name.equals(CandidateNativeLibrary.class.getName())) {
          synchronized (getClassLoadingLock(name)) {
            Class<?> loaded = findLoadedClass(name);
            if (loaded == null) {
              loaded = findClass(name);
            }
            if (resolve) {
              resolveClass(loaded);
            }
            return loaded;
          }
        }
        return super.loadClass(name, resolve);
      }
    };
  }

  private CandidateNativeLibrary candidateJarWith(String nativePath, String contents) throws Exception {
    Path path = jar("candidate.jar", true, Collections.singletonMap(nativePath, contents));
    return openedCandidate(codeSource(path));
  }

  private CandidateNativeLibrary openedCandidate(CodeSource source) throws IOException {
    CandidateNativeLibrary candidate =
        CandidateNativeLibrary.fromCodeSource(source, CandidateNativeLibrary.class.getClassLoader());
    openedJars.add(candidate);
    return candidate;
  }

  private static CodeSource codeSource(Path jar) throws Exception {
    return new CodeSource(jar.toUri().toURL(), (Certificate[]) null);
  }

  private Path jar(String name, boolean candidate, Map<String, String> entries) throws IOException {
    Path path = tempDir.resolve(name);
    Manifest manifest = new Manifest();
    manifest.getMainAttributes().put(Attributes.Name.MANIFEST_VERSION, "1.0");
    if (candidate) {
      manifest.getMainAttributes().putValue("Snowflake-JDBC-FIPS-Candidate", "true");
    }
    try (OutputStream output = Files.newOutputStream(path);
        JarOutputStream jar = new JarOutputStream(output, manifest)) {
      for (Map.Entry<String, String> entry : entries.entrySet()) {
        jar.putNextEntry(new JarEntry(entry.getKey()));
        jar.write(entry.getValue().getBytes(StandardCharsets.UTF_8));
        jar.closeEntry();
      }
    }
    return path;
  }
}
