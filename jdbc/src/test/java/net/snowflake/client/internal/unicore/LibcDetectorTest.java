package net.snowflake.client.internal.unicore;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.NullAndEmptySource;
import org.junit.jupiter.params.provider.ValueSource;

/**
 * Unit tests for {@link LibcDetector}'s pure parsers. These cover the {@code gnu}/{@code musl}
 * selection that decides which Linux native library loads — a silent misdetection means the driver
 * fails to load on Alpine (or on glibc). Kept off {@link NativeLibraryLoader} so no native load is
 * triggered.
 */
public class LibcDetectorTest {

  // --- parseLddContent: /usr/bin/ldd is a glibc shell script or the musl loader (Alpine) ---

  @Test
  public void shouldDetectMuslFromLddSymlinkTarget() {
    // On Alpine /usr/bin/ldd is a symlink to the musl dynamic loader.
    assertEquals("musl", LibcDetector.parseLddContent("/lib/ld-musl-x86_64.so.1"));
  }

  @Test
  public void shouldDetectGnuFromGlibcLddScriptBanner() {
    // The glibc /usr/bin/ldd is a shell script whose header names the GNU C Library.
    assertEquals(
        "gnu",
        LibcDetector.parseLddContent("#! /bin/bash\n# This file is part of the GNU C Library.\n"));
  }

  @ParameterizedTest
  @ValueSource(strings = {"muslib", "muscle", "amusing"})
  public void shouldNotMatchMuslSubstringWithoutWordBoundary(String content) {
    // Word-boundary guard: "musl" inside a larger word is not a libc marker.
    assertNull(LibcDetector.parseLddContent(content));
  }

  @Test
  public void shouldReturnNullFromLddContentWithNoMarker() {
    assertNull(LibcDetector.parseLddContent("some unrelated script contents"));
  }

  @ParameterizedTest
  @NullAndEmptySource
  public void shouldReturnNullFromNullOrEmptyLddContent(String content) {
    assertNull(LibcDetector.parseLddContent(content));
  }

  // --- parseCommandOutput: line 0 = `getconf GNU_LIBC_VERSION`, line 1 = `ldd --version` ---

  @Test
  public void shouldDetectGnuFromGetconfGlibcVersion() {
    assertEquals(
        "gnu",
        LibcDetector.parseCommandOutput("glibc 2.31\nldd (Ubuntu GLIBC 2.31-0ubuntu9) 2.31"));
  }

  @Test
  public void shouldDetectMuslFromLddVersionWhenGetconfFails() {
    // On Alpine `getconf GNU_LIBC_VERSION` errors (guarded by `|| true`); musl surfaces on line 1.
    assertEquals(
        "musl",
        LibcDetector.parseCommandOutput(
            "getconf: GNU_LIBC_VERSION: unknown variable\nmusl libc (x86_64)"));
  }

  @Test
  public void shouldPreferGnuGetconfMarkerOverLaterLines() {
    assertEquals("gnu", LibcDetector.parseCommandOutput("glibc 2.35\nmusl libc (x86_64)"));
  }

  @Test
  public void shouldReturnNullWhenMuslMarkerIsOnlyOnGetconfLine() {
    // musl is only trusted on the ldd line (index 1); a stray marker on line 0 is not conclusive.
    assertNull(LibcDetector.parseCommandOutput("musl\nunrelated second line"));
  }

  @Test
  public void shouldReturnNullFromSingleLineWithoutMarkers() {
    assertNull(LibcDetector.parseCommandOutput("command not found"));
  }

  @ParameterizedTest
  @NullAndEmptySource
  public void shouldReturnNullFromNullOrEmptyCommandOutput(String output) {
    assertNull(LibcDetector.parseCommandOutput(output));
  }
}
