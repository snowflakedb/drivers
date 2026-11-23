package net.snowflake.client.internal.unicore;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.stream.Stream;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.condition.EnabledOnOs;
import org.junit.jupiter.api.condition.OS;
import org.junit.jupiter.api.io.TempDir;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

/**
 * Unit tests for {@link BundledNativeLibrary}. Resources are injected, so no native library is
 * loaded and no FIPS build is needed.
 */
public class BundledNativeLibraryTest {

  private static final String RESOURCE_DIR = "/native/darwin-aarch64/";
  private static final String LIBRARY = "libjdbc_bridge.dylib";
  private static final String CRYPTO = "libaws_lc_fips_0_13_17_crypto.dylib";
  private static final String RUST_WRAPPER = "libaws_lc_fips_0_13_17_rust_wrapper.dylib";

  @TempDir Path extractionDir;

  private final Map<String, byte[]> resources = new HashMap<>();

  @BeforeEach
  void addLibraryResource() {
    putResource(LIBRARY, "bridge");
  }

  @Test
  public void shouldParseCompanionNamesInManifestOrder() throws IOException {
    List<String> names =
        BundledNativeLibrary.parseCompanions(
            "\n  " + CRYPTO + "  \r\n\r\n" + RUST_WRAPPER + "\n\n");

    assertEquals(Arrays.asList(CRYPTO, RUST_WRAPPER), names);
  }

  @ParameterizedTest
  @ValueSource(strings = {"", "\n", " \r\n \n"})
  public void shouldParseBlankManifestAsNoCompanions(String manifest) throws IOException {
    assertEquals(Collections.emptyList(), BundledNativeLibrary.parseCompanions(manifest));
  }

  @ParameterizedTest
  @ValueSource(strings = {"../escape.dylib", "sub/lib.dylib", "sub\\lib.dll", "..", "."})
  public void shouldRejectManifestEntryWithDirectoryComponent(String entry) {
    IOException e =
        assertThrows(IOException.class, () -> BundledNativeLibrary.parseCompanions(entry));

    assertTrue(e.getMessage().contains("'" + entry + "'"), e.getMessage());
  }

  @Test
  @EnabledOnOs(OS.WINDOWS)
  public void shouldRejectWindowsDriveRelativeManifestEntry() {
    IOException e =
        assertThrows(IOException.class, () -> BundledNativeLibrary.parseCompanions("C:foo"));

    assertTrue(e.getMessage().contains("'C:foo'"), e.getMessage());
  }

  @Test
  public void shouldLoadOnlyLibraryWhenManifestIsMissing() throws IOException {
    BundledNativeLibrary bundle = read();
    bundle.copyInto(extractionDir);

    assertEquals(Collections.singletonList(LIBRARY), bundle.loadOrder());
    assertEquals(Collections.singletonList(LIBRARY), listExtracted());
  }

  @Test
  public void shouldLoadCompanionsInManifestOrderBeforeLibrary() throws IOException {
    putResource(BundledNativeLibrary.COMPANIONS_MANIFEST, RUST_WRAPPER + "\n" + CRYPTO + "\n");

    assertEquals(Arrays.asList(RUST_WRAPPER, CRYPTO, LIBRARY), read().loadOrder());
  }

  @Test
  public void shouldCopyLibraryAndCompanionsIntoOneDirectory() throws IOException {
    putResource(CRYPTO, "crypto");
    putResource(RUST_WRAPPER, "wrapper");
    putResource(BundledNativeLibrary.COMPANIONS_MANIFEST, CRYPTO + "\n" + RUST_WRAPPER);

    read().copyInto(extractionDir);

    assertEquals(Arrays.asList(CRYPTO, RUST_WRAPPER, LIBRARY), listExtracted());
    assertArrayEquals(bytes("bridge"), Files.readAllBytes(extractionDir.resolve(LIBRARY)));
    assertArrayEquals(bytes("crypto"), Files.readAllBytes(extractionDir.resolve(CRYPTO)));
    assertArrayEquals(bytes("wrapper"), Files.readAllBytes(extractionDir.resolve(RUST_WRAPPER)));
  }

  @Test
  public void shouldThrowNamingResourceWhenListedCompanionIsMissing() throws IOException {
    putResource(CRYPTO, "crypto");
    putResource(BundledNativeLibrary.COMPANIONS_MANIFEST, CRYPTO + "\n" + RUST_WRAPPER);
    BundledNativeLibrary bundle = read();

    IOException e = assertThrows(IOException.class, () -> bundle.copyInto(extractionDir));

    assertTrue(e.getMessage().contains(RESOURCE_DIR + RUST_WRAPPER), e.getMessage());
  }

  @Test
  public void shouldThrowNamingResourceWhenLibraryIsMissing() throws IOException {
    resources.clear();
    BundledNativeLibrary bundle = read();

    IOException e = assertThrows(IOException.class, () -> bundle.copyInto(extractionDir));

    assertTrue(e.getMessage().contains(RESOURCE_DIR + LIBRARY), e.getMessage());
  }

  @Test
  public void shouldThrowOnReadWhenManifestEntryHasDirectoryComponent() {
    putResource(BundledNativeLibrary.COMPANIONS_MANIFEST, "../" + CRYPTO);

    IOException e = assertThrows(IOException.class, this::read);

    assertTrue(e.getMessage().contains("'../" + CRYPTO + "'"), e.getMessage());
  }

  private BundledNativeLibrary read() throws IOException {
    return BundledNativeLibrary.read(RESOURCE_DIR, LIBRARY, this::openResource);
  }

  private InputStream openResource(String resourcePath) {
    if (!resourcePath.startsWith(RESOURCE_DIR)) {
      return null;
    }
    byte[] content = resources.get(resourcePath.substring(RESOURCE_DIR.length()));
    return content == null ? null : new ByteArrayInputStream(content);
  }

  private void putResource(String name, String content) {
    resources.put(name, bytes(content));
  }

  private List<String> listExtracted() throws IOException {
    List<String> names = new ArrayList<>();
    try (Stream<Path> files = Files.list(extractionDir)) {
      files.forEach(p -> names.add(p.getFileName().toString()));
    }
    Collections.sort(names);
    return names;
  }

  private static byte[] bytes(String content) {
    return content.getBytes(StandardCharsets.UTF_8);
  }
}
