package net.snowflake.client.internal.unicore;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.io.OutputStream;
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
    IOException error = assertThrows(IOException.class, () -> CandidateNativeLibrary.fromCodeSource(null));
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
  public void shouldIgnoreCandidateMarkerInAnotherClasspathJar() throws Exception {
    Path ordinary = jar("ordinary.jar", false, Collections.emptyMap());
    Path candidate = jar("candidate.jar", true, Collections.singletonMap(NATIVE_PATH, "foreign-native"));
    ClassLoader oldContextLoader = Thread.currentThread().getContextClassLoader();
    try (URLClassLoader classpath = new URLClassLoader(new java.net.URL[] {candidate.toUri().toURL()})) {
      Thread.currentThread().setContextClassLoader(classpath);
      assertNull(CandidateNativeLibrary.fromCodeSource(codeSource(ordinary)));
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
    assertNull(CandidateNativeLibrary.fromCodeSource(codeSource(ordinary)));
  }

  private CandidateNativeLibrary candidateJarWith(String nativePath, String contents) throws Exception {
    Path path = jar("candidate.jar", true, Collections.singletonMap(nativePath, contents));
    return openedCandidate(codeSource(path));
  }

  private CandidateNativeLibrary openedCandidate(CodeSource source) throws IOException {
    CandidateNativeLibrary candidate = CandidateNativeLibrary.fromCodeSource(source);
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
