package net.snowflake.client.internal.unicore;

import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assertions.fail;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

import java.io.BufferedReader;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.nio.channels.FileChannel;
import java.nio.channels.FileLock;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardOpenOption;
import java.nio.file.attribute.FileTime;
import java.nio.file.attribute.UserPrincipal;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ThreadFactory;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

class NativeLibraryTempDirsTest {

  private static final byte[] LIBRARY_BYTES = {1, 2, 3};

  @TempDir Path tmpDir;

  private Path ownDir;
  private UserPrincipal owner;

  @BeforeEach
  void createOwnExtract() throws IOException {
    ownDir = Files.createTempDirectory(tmpDir, NativeLibraryTempDirs.DIR_PREFIX + "own");
    owner = Files.getOwner(ownDir);
  }

  @Test
  void shouldDeleteOldExtractWhoseLockIsFree() throws Exception {
    Path stale = staleExtract();
    Files.createFile(stale.resolve(NativeLibraryTempDirs.LOCK_NAME));
    backdate(stale);

    NativeLibraryTempDirs.deleteStaleExtracts(ownDir, owner);

    assertFalse(Files.exists(stale));
  }

  @Test
  void shouldDeleteOldLocklessExtract() throws Exception {
    Path stale = staleExtract();
    backdate(stale);

    NativeLibraryTempDirs.deleteStaleExtracts(ownDir, owner);

    assertFalse(Files.exists(stale));
  }

  @Test
  void shouldTreatAStaleExtractDeletedDuringReclaimAsRemoved() throws Exception {
    Path stale = staleExtract();
    backdate(stale);
    NativeLibraryTempDirs.deleteStaleExtracts(ownDir, owner);
    assertFalse(Files.exists(stale));

    NativeLibraryTempDirs.deleteIfReclaimable(stale, owner);

    assertFalse(Files.exists(stale));
  }

  @Test
  void shouldKeepRecentExtractEvenWhenItsLockIsFree() throws Exception {
    Path starting = staleExtract();
    Files.createFile(starting.resolve(NativeLibraryTempDirs.LOCK_NAME));

    NativeLibraryTempDirs.deleteStaleExtracts(ownDir, owner);

    assertTrue(Files.isDirectory(starting));
  }

  @Test
  void shouldKeepRecentLocklessExtract() throws Exception {
    Path starting = staleExtract();

    NativeLibraryTempDirs.deleteStaleExtracts(ownDir, owner);

    assertTrue(Files.isDirectory(starting));
  }

  @Test
  void shouldKeepOldExtractWhoseLockIsHeldByAnotherChannelInThisJvm() throws Exception {
    Path live = staleExtract();
    try (FileChannel channel = openLock(live);
        FileLock held = channel.lock()) {
      backdate(live);

      NativeLibraryTempDirs.deleteStaleExtracts(ownDir, owner);

      assertTrue(Files.isDirectory(live));
    }
  }

  @Test
  void shouldKeepOldExtractWhoseLockIsHeldByAnotherProcess() throws Exception {
    Path live = staleExtract();
    try (LockHolder holder = LockHolder.start(live.resolve(NativeLibraryTempDirs.LOCK_NAME))) {
      backdate(live);

      NativeLibraryTempDirs.deleteStaleExtracts(ownDir, owner);

      assertTrue(Files.isDirectory(live));
    }
  }

  @Test
  void shouldKeepOwnExtractDirectory() throws Exception {
    backdate(ownDir);

    NativeLibraryTempDirs.deleteStaleExtracts(ownDir, owner);

    assertTrue(Files.isDirectory(ownDir));
  }

  @Test
  void shouldKeepExtractOwnedByAnotherUser() throws Exception {
    Path foreign = staleExtract();
    backdate(foreign);
    UserPrincipal someoneElse = () -> "someone-else";

    NativeLibraryTempDirs.deleteStaleExtracts(ownDir, someoneElse);

    assertTrue(Files.isDirectory(foreign));
  }

  @Test
  void shouldIgnoreEntriesWithoutTheNativePrefix() throws Exception {
    Path other = Files.createTempDirectory(tmpDir, "other-");
    Files.write(other.resolve("keep"), LIBRARY_BYTES);
    backdate(other);

    NativeLibraryTempDirs.deleteStaleExtracts(ownDir, owner);

    assertTrue(Files.exists(other.resolve("keep")));
  }

  @Test
  void shouldDeleteNestedDirectoriesInsideAStaleExtract() throws Exception {
    Path stale = staleExtract();
    Path nested = Files.createDirectories(stale.resolve("a").resolve("b"));
    Files.write(nested.resolve("file"), LIBRARY_BYTES);
    backdate(stale);

    NativeLibraryTempDirs.deleteStaleExtracts(ownDir, owner);

    assertFalse(Files.exists(stale));
  }

  @Test
  void shouldNotFollowACandidateThatIsASymlink() throws Exception {
    Path victim = victimDirectory();
    Path link = tmpDir.resolve(NativeLibraryTempDirs.DIR_PREFIX + "link");
    createSymlinkOrSkip(link, victim);

    NativeLibraryTempDirs.deleteStaleExtracts(ownDir, owner);

    assertTrue(Files.exists(victim.resolve("precious")));
    assertTrue(Files.isSymbolicLink(link));
  }

  @Test
  void shouldDeleteASymlinkInsideAStaleExtractWithoutFollowingIt() throws Exception {
    Path victim = victimDirectory();
    Path stale = staleExtract();
    Path link = stale.resolve("link");
    createSymlinkOrSkip(link, victim);
    backdate(stale);

    NativeLibraryTempDirs.deleteStaleExtracts(ownDir, owner);

    assertFalse(Files.exists(stale));
    assertTrue(Files.exists(victim.resolve("precious")));
  }

  @Test
  void shouldReturnNoLockWhenTheLockIsAlreadyHeld() throws Exception {
    Path lockPath = ownDir.resolve(NativeLibraryTempDirs.LOCK_NAME);
    try (FileChannel channel = openLock(ownDir);
        FileLock held = channel.lock()) {
      assertNull(NativeLibraryTempDirs.tryLock(lockPath));
    }
  }

  @Test
  void shouldReturnNoLockWhenAnotherProcessHoldsIt() throws Exception {
    Path lockPath = ownDir.resolve(NativeLibraryTempDirs.LOCK_NAME);
    try (LockHolder holder = LockHolder.start(lockPath)) {
      assertNull(NativeLibraryTempDirs.tryLock(lockPath));
    }
  }

  @Test
  void shouldExtractOnlyTheLockAndTheLibrary() throws Exception {
    try (NativeLibraryTempDirs.ExtractDir extracted =
        NativeLibraryLoader.extractTo(new ByteArrayInputStream(LIBRARY_BYTES), tmpDir, "test")) {
      assertNotNull(extracted.lock);
      assertEquals(
          new HashSet<>(
              Arrays.asList(
                  NativeLibraryTempDirs.LOCK_NAME, NativeLibraryLoader.nativeLibFileName())),
          childNames(extracted.dir));
    }
  }

  @Test
  void shouldReleaseTheLockWhenTheLibraryCopyFails() throws Exception {
    InputStream failing =
        new InputStream() {
          @Override
          public int read() throws IOException {
            throw new IOException("copy failed");
          }
        };

    SFSQLException thrown =
        assertThrows(
            SFSQLException.class, () -> NativeLibraryLoader.extractTo(failing, tmpDir, "copyfail"));

    assertEquals(ErrorCode.NATIVE_LIBRARY_LOAD_FAILED, thrown.getErrorCode());
    Path failedExtract = onlyChildWithPrefix(tmpDir, NativeLibraryTempDirs.DIR_PREFIX + "copyfail");
    FileLock lock =
        NativeLibraryTempDirs.tryLock(failedExtract.resolve(NativeLibraryTempDirs.LOCK_NAME));
    assertNotNull(lock);
    lock.channel().close();
  }

  @Test
  void shouldReleaseTheLockWhenSystemLoadFails() throws Exception {
    NativeLibraryTempDirs.ExtractDir extracted =
        NativeLibraryLoader.extractTo(new ByteArrayInputStream(LIBRARY_BYTES), tmpDir, "loadfail");

    SFSQLException thrown =
        assertThrows(SFSQLException.class, () -> NativeLibraryLoader.loadExtracted(extracted));

    assertEquals(ErrorCode.NATIVE_LIBRARY_LOAD_FAILED, thrown.getErrorCode());
    FileLock lock =
        NativeLibraryTempDirs.tryLock(extracted.dir.resolve(NativeLibraryTempDirs.LOCK_NAME));
    assertNotNull(lock);
    lock.channel().close();
  }

  @Test
  void shouldNotThrowWhenTheCleanupThreadCannotBeStarted() {
    ThreadFactory exhausted =
        cleanup -> {
          throw new OutOfMemoryError("unable to create native thread");
        };

    assertDoesNotThrow(
        () -> NativeLibraryTempDirs.deleteStaleExtractsInBackground(ownDir, exhausted));
  }

  @Test
  void shouldLeaveNothingBehindAfterACleanJvmExit() throws Exception {
    Path root = Files.createDirectory(tmpDir.resolve("exit-root"));
    Process child = startChild("extract-and-exit", root.toString());
    child.getOutputStream().close();
    if (!child.waitFor(30, TimeUnit.SECONDS)) {
      child.destroyForcibly();
      fail("child JVM did not exit; output: " + readAll(child));
    }
    assertEquals(0, child.exitValue(), "child output: " + readAll(child));

    assertEquals(new HashSet<String>(), childNames(root));
  }

  private Path staleExtract() throws IOException {
    Path dir = Files.createTempDirectory(tmpDir, NativeLibraryTempDirs.DIR_PREFIX);
    Files.write(dir.resolve("libjdbc_bridge.so"), LIBRARY_BYTES);
    return dir;
  }

  private Path victimDirectory() throws IOException {
    Path victim = Files.createTempDirectory(tmpDir, "victim-");
    Files.write(victim.resolve("precious"), LIBRARY_BYTES);
    backdate(victim);
    return victim;
  }

  private static FileChannel openLock(Path dir) throws IOException {
    return FileChannel.open(
        dir.resolve(NativeLibraryTempDirs.LOCK_NAME),
        StandardOpenOption.CREATE,
        StandardOpenOption.READ,
        StandardOpenOption.WRITE);
  }

  private static void backdate(Path dir) throws IOException {
    Files.setLastModifiedTime(
        dir,
        FileTime.fromMillis(
            System.currentTimeMillis() - NativeLibraryTempDirs.MIN_AGE_MILLIS - 1000));
  }

  private static void createSymlinkOrSkip(Path link, Path target) {
    try {
      Files.createSymbolicLink(link, target);
    } catch (IOException | UnsupportedOperationException e) {
      assumeTrue(false, "symlinks are not available here: " + e);
    }
  }

  private static Path onlyChildWithPrefix(Path dir, String prefix) throws IOException {
    try (Stream<Path> children = Files.list(dir)) {
      List<Path> matches =
          children
              .filter(p -> p.getFileName().toString().startsWith(prefix))
              .collect(Collectors.toList());
      assertEquals(1, matches.size(), "directories starting with " + prefix + ": " + matches);
      return matches.get(0);
    }
  }

  private static Set<String> childNames(Path dir) throws IOException {
    try (Stream<Path> children = Files.list(dir)) {
      return children.map(p -> p.getFileName().toString()).collect(Collectors.toSet());
    }
  }

  private static Process startChild(String mode, String path) throws IOException {
    List<String> cmd = new ArrayList<>();
    cmd.add(Paths.get(System.getProperty("java.home"), "bin", "java").toString());
    cmd.add("-cp");
    cmd.add(System.getProperty("java.class.path"));
    cmd.add(ChildProcess.class.getName());
    cmd.add(mode);
    cmd.add(path);
    return new ProcessBuilder(cmd).redirectErrorStream(true).start();
  }

  private static String awaitLine(Process child) throws Exception {
    CompletableFuture<String> line = new CompletableFuture<>();
    Thread reader =
        new Thread(
            () -> {
              try {
                line.complete(
                    new BufferedReader(
                            new InputStreamReader(child.getInputStream(), StandardCharsets.UTF_8))
                        .readLine());
              } catch (IOException e) {
                line.completeExceptionally(e);
              }
            });
    reader.setDaemon(true);
    reader.start();
    try {
      return line.get(30, TimeUnit.SECONDS);
    } catch (TimeoutException e) {
      child.destroyForcibly();
      throw e;
    }
  }

  /** A child JVM that holds the lock on a file until closed. */
  private static final class LockHolder implements AutoCloseable {
    private final Process child;

    private LockHolder(Process child) {
      this.child = child;
    }

    static LockHolder start(Path lockPath) throws Exception {
      Process child = startChild("hold-lock", lockPath.toString());
      try {
        assertEquals("locked", awaitLine(child));
        return new LockHolder(child);
      } catch (Throwable t) {
        child.destroyForcibly();
        throw t;
      }
    }

    @Override
    public void close() throws Exception {
      child.getOutputStream().close();
      if (!child.waitFor(30, TimeUnit.SECONDS)) {
        child.destroyForcibly();
        fail("lock-holder JVM did not exit after stdin closed");
      }
    }
  }

  private static String readAll(Process child) throws IOException {
    try (BufferedReader reader =
        new BufferedReader(new InputStreamReader(child.getInputStream(), StandardCharsets.UTF_8))) {
      return reader.lines().collect(Collectors.joining("\n"));
    }
  }

  /**
   * Runs in a separate JVM so locks and {@code deleteOnExit} behave as they do across processes.
   */
  static final class ChildProcess {
    public static void main(String[] args) throws Exception {
      Path path = Paths.get(args[1]);
      if ("hold-lock".equals(args[0])) {
        FileLock lock = NativeLibraryTempDirs.tryLock(path);
        System.out.println(lock != null ? "locked" : "not-locked");
        System.out.flush();
        while (System.in.read() != -1) {
          // Hold the lock until the parent closes stdin.
        }
      } else {
        NativeLibraryLoader.extractTo(new ByteArrayInputStream(LIBRARY_BYTES), path, "test");
      }
    }
  }
}
