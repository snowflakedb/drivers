package net.snowflake.client.internal.unicore;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

class NativeLibraryLoaderTest {

  @Test
  void shouldMapLinuxAarch64ToMavenClassifier() {
    assertEquals("linux-aarch_64", NativeLibraryLoader.mavenClassifier("linux", "aarch64"));
  }

  @Test
  void shouldMapDarwinX86ToOsxClassifier() {
    assertEquals("osx-x86_64", NativeLibraryLoader.mavenClassifier("darwin", "x86_64"));
  }

  @Test
  void shouldNameMissingNativeResourceAndSuggestedClassifier() {
    String message =
        NativeLibraryLoader.missingNativeMessage(
            "/native/linux-aarch64-gnu/libjdbc_bridge.so", "Linux", "aarch64", "linux-aarch_64");
    assertTrue(message.contains("linux-aarch_64"));
    assertTrue(message.contains("Linux/aarch64"));
    assertTrue(message.contains("snowflake-jdbc-native-fips:<version>:linux-aarch_64"));
    assertTrue(message.contains("snowflake-jdbc-native-all-fips"));
    assertFalse(message.contains("classifier-less"));
    assertFalse(message.contains("CORE_PATH"));
    assertFalse(message.contains("jdbc.library.path"));
    assertFalse(message.contains("github.com"));
  }

  @Test
  void shouldNameExtractFailureCauseAndTmpdirOverride() {
    String message = NativeLibraryLoader.extractFailedMessage("/tmp", "Read-only file system");
    assertTrue(message.contains("Read-only file system"));
    assertTrue(message.contains("-Djava.io.tmpdir"));
    assertFalse(message.contains("CORE_PATH"));
    assertFalse(message.contains("jdbc.library.path"));
    assertFalse(message.contains("github.com"));
  }

  @Test
  void shouldExplainGlibcOnlyJarOnMuslHost() {
    String message =
        NativeLibraryLoader.glibcOnlyMessage("/native/linux-x86_64-musl/libjdbc_bridge.so");
    assertTrue(message.contains("/native/linux-x86_64-musl/libjdbc_bridge.so"));
    assertTrue(message.contains("only the glibc native"));
    assertTrue(message.contains("-Djdbc.libc=gnu"));
    assertFalse(message.contains("FIPS"));
    assertFalse(message.contains("CORE_PATH"));
    assertFalse(message.contains("github.com"));
  }

  @Test
  void shouldTreatMuslHostWithGlibcNativeAsGlibcOnlyJar() {
    assertTrue(NativeLibraryLoader.onlyGlibcNativeBundled("linux", "musl", true));
    assertFalse(NativeLibraryLoader.onlyGlibcNativeBundled("linux", "musl", false));
    assertFalse(NativeLibraryLoader.onlyGlibcNativeBundled("linux", "gnu", true));
    assertFalse(NativeLibraryLoader.onlyGlibcNativeBundled("darwin", "musl", true));
  }

  @Test
  void shouldNameManifestFailureCauseWithoutTmpdirHint() {
    String message =
        NativeLibraryLoader.manifestFailedMessage(
            "/native/darwin-aarch64/libjdbc_bridge.dylib", "Invalid entry '../x.dylib'");
    assertTrue(message.contains("/native/darwin-aarch64/libjdbc_bridge.dylib"));
    assertTrue(message.contains("Invalid entry '../x.dylib'"));
    assertFalse(message.contains("java.io.tmpdir"));
    assertFalse(message.contains("github.com"));
  }

  @Test
  void shouldNameLoadFailureCauseAndExecHints() {
    String message =
        NativeLibraryLoader.loadFailedMessage(
            "/tmp/libjdbc_bridge.so", "failed to map segment from shared object");
    assertTrue(message.contains("failed to map segment"));
    assertTrue(message.contains("noexec"));
    assertTrue(message.contains("glibc 2.28"));
    assertTrue(message.contains("libgcc"));
    assertFalse(message.contains("CORE_PATH"));
    assertFalse(message.contains("jdbc.library.path"));
    assertFalse(message.contains("github.com"));
  }
}
