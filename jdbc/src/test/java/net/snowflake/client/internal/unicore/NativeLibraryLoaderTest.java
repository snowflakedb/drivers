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
