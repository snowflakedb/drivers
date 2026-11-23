package net.snowflake.client.internal.unicore;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.InvalidPathException;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.function.Function;

/**
 * The files one JAR resource directory carries for a native library: the companion libraries listed
 * in its {@value #COMPANIONS_MANIFEST}, then the library itself, in load order.
 *
 * <p>The manifest lists companion file names, one per line. JAR resources cannot be listed at
 * runtime, so the manifest is the only way to discover them. A missing manifest means the library
 * has no companions. Companions load before the library: Windows does not search the loading DLL's
 * directory for its dependencies, and macOS then resolves them against images that are already
 * loaded.
 */
final class BundledNativeLibrary {

  static final String COMPANIONS_MANIFEST = "companions.txt";

  private final String resourceDir;
  private final Function<String, InputStream> resourceOpener;
  private final List<String> loadOrder;

  private BundledNativeLibrary(
      String resourceDir, Function<String, InputStream> resourceOpener, List<String> loadOrder) {
    this.resourceDir = resourceDir;
    this.resourceOpener = resourceOpener;
    this.loadOrder = loadOrder;
  }

  /**
   * Reads the companion manifest in {@code resourceDir}.
   *
   * @param resourceDir resource directory ending in {@code /}
   * @param libFileName file name of the library, loaded after its companions
   * @param resourceOpener returns the resource stream, or {@code null} when the resource is absent
   * @throws IOException when the manifest cannot be read or an entry is not a bare file name
   */
  static BundledNativeLibrary read(
      String resourceDir, String libFileName, Function<String, InputStream> resourceOpener)
      throws IOException {
    List<String> loadOrder = new ArrayList<>();
    try (InputStream manifest = resourceOpener.apply(resourceDir + COMPANIONS_MANIFEST)) {
      if (manifest != null) {
        loadOrder.addAll(parseCompanions(readUtf8(manifest)));
      }
    }
    loadOrder.add(libFileName);
    return new BundledNativeLibrary(
        resourceDir, resourceOpener, Collections.unmodifiableList(loadOrder));
  }

  /** File names in load order: companions in manifest order, then the library. */
  List<String> loadOrder() {
    return loadOrder;
  }

  /**
   * Copies every file in {@link #loadOrder()} into {@code dir}, registering each for {@code
   * deleteOnExit} before it is written.
   *
   * @throws IOException when a file is missing from the JAR or cannot be written
   */
  void copyInto(Path dir) throws IOException {
    for (String name : loadOrder) {
      copy(name, dir.resolve(name));
    }
  }

  /**
   * Parses a companion manifest: one bare file name per line, surrounding whitespace and blank
   * lines ignored.
   *
   * @throws IOException when an entry is not a bare file name
   */
  static List<String> parseCompanions(String manifest) throws IOException {
    List<String> names = new ArrayList<>();
    for (String line : manifest.split("\r?\n")) {
      String name = line.trim();
      if (name.isEmpty()) {
        continue;
      }
      if (!isBareFileName(name)) {
        throw new IOException(
            "Invalid entry '"
                + name
                + "' in "
                + COMPANIONS_MANIFEST
                + ": expected a file name without a directory component");
      }
      names.add(name);
    }
    return names;
  }

  private static boolean isBareFileName(String name) {
    if (name.indexOf('/') >= 0 || name.indexOf('\\') >= 0) {
      return false;
    }
    try {
      Path path = Paths.get(name);
      return path.getRoot() == null
          && path.getNameCount() == 1
          && !".".equals(name)
          && !"..".equals(name);
    } catch (InvalidPathException e) {
      return false;
    }
  }

  private void copy(String name, Path target) throws IOException {
    String resourcePath = resourceDir + name;
    try (InputStream in = resourceOpener.apply(resourcePath)) {
      if (in == null) {
        throw new IOException("Native library file missing from the JAR at " + resourcePath);
      }
      target.toFile().deleteOnExit();
      Files.copy(in, target, StandardCopyOption.REPLACE_EXISTING);
    }
  }

  private static String readUtf8(InputStream in) throws IOException {
    ByteArrayOutputStream out = new ByteArrayOutputStream();
    byte[] buffer = new byte[4096];
    int read;
    while ((read = in.read(buffer)) != -1) {
      out.write(buffer, 0, read);
    }
    return new String(out.toByteArray(), StandardCharsets.UTF_8);
  }
}
