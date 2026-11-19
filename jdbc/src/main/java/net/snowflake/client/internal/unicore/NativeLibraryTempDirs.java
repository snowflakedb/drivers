package net.snowflake.client.internal.unicore;

import java.io.Closeable;
import java.io.IOException;
import java.nio.channels.FileChannel;
import java.nio.channels.FileLock;
import java.nio.channels.OverlappingFileLockException;
import java.nio.file.DirectoryStream;
import java.nio.file.FileSystemException;
import java.nio.file.FileVisitResult;
import java.nio.file.Files;
import java.nio.file.LinkOption;
import java.nio.file.NoSuchFileException;
import java.nio.file.Path;
import java.nio.file.SimpleFileVisitor;
import java.nio.file.StandardOpenOption;
import java.nio.file.attribute.BasicFileAttributes;
import java.nio.file.attribute.UserPrincipal;
import java.util.concurrent.ThreadFactory;
import java.util.concurrent.TimeUnit;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;

/**
 * Per-JVM extract directory under {@code java.io.tmpdir}, and reclaim of copies left by a JVM that
 * did not run {@code deleteOnExit} (crash, {@code kill -9}, Windows holding a loaded DLL).
 *
 * <p>Each extract holds an exclusive lock on {@code .lock} when the file system supports it. The OS
 * releases that lock when the process dies. After the library loads, a background thread deletes
 * sibling {@code snowflake-jdbc-native-*} directories that are real directories (not symlinks),
 * owned by the same user as this JVM's extract, older than {@link #MIN_AGE_MILLIS}, and whose
 * {@code .lock} is free or absent. The age check keeps a directory another JVM is still creating.
 *
 * <p>The reclaim check and the delete are separate steps. The delete never follows symlinks, so a
 * candidate swapped for a link between them loses only the link. On a sticky temp directory (Linux
 * and macOS {@code /tmp}), only an entry's owner can rename or replace it, and candidates are
 * required to be owned by the user running this JVM.
 *
 * <p>Checking whether a candidate's lock is free opens a second channel on its {@code .lock}. When
 * another class loader in the same JVM holds that lock, {@code tryLock} throws {@link
 * OverlappingFileLockException} and closing the probe channel can release every lock this JVM holds
 * on that file on systems with per-process locks (Linux, macOS). That arises only when several
 * class loaders in one JVM each load the driver, as in an application server with the driver in
 * several web apps. The directory they then lose protection for is deleted only by a later cleanup
 * in another JVM; a library that is already loaded keeps working after its file is deleted on Linux
 * and macOS, and Windows locks are per handle.
 */
final class NativeLibraryTempDirs {

  static final String DIR_PREFIX = "snowflake-jdbc-native-";
  static final String LOCK_NAME = ".lock";
  static final long MIN_AGE_MILLIS = TimeUnit.MINUTES.toMillis(10);

  private static final SFLogger logger =
      SFLoggerFactory.getDeliveryLogger(NativeLibraryTempDirs.class.getName());

  private NativeLibraryTempDirs() {}

  /**
   * A created extract directory and the lock that marks it live; the lock is null if unavailable.
   */
  static final class ExtractDir implements Closeable {
    final Path dir;
    final FileLock lock;

    ExtractDir(Path dir, FileLock lock) {
      this.dir = dir;
      this.lock = lock;
    }

    @Override
    public void close() throws IOException {
      if (lock != null) {
        lock.channel().close();
      }
    }
  }

  /**
   * Creates a mode-700 extract directory under {@code tmpRoot} and locks its {@code .lock}. The
   * directory and {@code .lock} are registered for {@code deleteOnExit} in that order, so a file
   * the caller registers afterwards is deleted first and the directory is empty when its turn
   * comes. A lock failure is logged and the directory is returned without one.
   */
  static ExtractDir create(Path tmpRoot, String version) throws IOException {
    Path dir = Files.createTempDirectory(tmpRoot, DIR_PREFIX + version);
    dir.toFile().deleteOnExit();
    Path lockPath = dir.resolve(LOCK_NAME);
    lockPath.toFile().deleteOnExit();
    return new ExtractDir(dir, tryLock(lockPath));
  }

  /** Returns an exclusive lock on {@code lockPath}, or null if it is held or unsupported. */
  static FileLock tryLock(Path lockPath) {
    FileChannel channel;
    try {
      channel =
          FileChannel.open(
              lockPath,
              StandardOpenOption.CREATE,
              StandardOpenOption.READ,
              StandardOpenOption.WRITE);
    } catch (IOException e) {
      logRecoverableFailure("Could not open native-library lock file " + lockPath, e);
      return null;
    }
    try {
      FileLock lock = channel.tryLock();
      if (lock != null) {
        return lock;
      }
      logger.warn("Native-library lock file {} is held by another process", lockPath);
    } catch (IOException | OverlappingFileLockException e) {
      logRecoverableFailure("Could not lock native-library lock file " + lockPath, e);
    }
    closeQuietly(channel);
    return null;
  }

  static void deleteStaleExtractsInBackground(Path ownDir) {
    deleteStaleExtractsInBackground(ownDir, NativeLibraryTempDirs::newCleanerThread);
  }

  /**
   * Starts the cleanup on a thread from {@code threadFactory}. A thread that cannot be created or
   * started (for example {@link OutOfMemoryError} under a container pid limit) is logged and
   * skipped, so cleanup never decides whether the native library loaded.
   */
  static void deleteStaleExtractsInBackground(Path ownDir, ThreadFactory threadFactory) {
    try {
      threadFactory.newThread(() -> deleteStaleExtracts(ownDir)).start();
    } catch (RuntimeException | OutOfMemoryError e) {
      logRecoverableFailure("Could not start stale JDBC native-library extract cleanup", e);
    }
  }

  private static Thread newCleanerThread(Runnable cleanup) {
    Thread cleaner = new Thread(cleanup, "snowflake-jdbc-native-cleanup");
    cleaner.setDaemon(true);
    return cleaner;
  }

  static void deleteStaleExtracts(Path ownDir) {
    UserPrincipal owner;
    try {
      owner = Files.getOwner(ownDir, LinkOption.NOFOLLOW_LINKS);
    } catch (IOException | UnsupportedOperationException e) {
      logRecoverableFailure(
          "Could not read the owner of " + ownDir + "; skipping stale extract cleanup", e);
      return;
    }
    deleteStaleExtracts(ownDir, owner);
  }

  /** Deletes reclaimable siblings of {@code ownDir}. Failures are logged and skipped. */
  static void deleteStaleExtracts(Path ownDir, UserPrincipal owner) {
    Path tmpRoot = ownDir.getParent();
    if (tmpRoot == null) {
      return;
    }
    try (DirectoryStream<Path> stream = Files.newDirectoryStream(tmpRoot, DIR_PREFIX + "*")) {
      for (Path candidate : stream) {
        if (!candidate.equals(ownDir)) {
          deleteIfReclaimable(candidate, owner);
        }
      }
    } catch (IOException e) {
      logRecoverableFailure("Could not list native-library temp directories under " + tmpRoot, e);
    }
  }

  static void deleteIfReclaimable(Path candidate, UserPrincipal owner) {
    try {
      if (isReclaimable(candidate, owner)) {
        deleteTree(candidate);
        logger.debug("Removed stale JDBC native-library extract {}", candidate);
      }
    } catch (NoSuchFileException e) {
      return;
    } catch (FileSystemException e) {
      logger.debug(
          "Could not remove stale JDBC native-library extract {}: {}",
          candidate,
          e.getClass().getName());
    } catch (IOException | UnsupportedOperationException e) {
      logRecoverableFailure("Could not remove stale JDBC native-library extract " + candidate, e);
    }
  }

  private static boolean isReclaimable(Path candidate, UserPrincipal owner) throws IOException {
    BasicFileAttributes attrs =
        Files.readAttributes(candidate, BasicFileAttributes.class, LinkOption.NOFOLLOW_LINKS);
    if (!attrs.isDirectory()) {
      return false;
    }
    if (!owner.equals(Files.getOwner(candidate, LinkOption.NOFOLLOW_LINKS))) {
      return false;
    }
    if (System.currentTimeMillis() - attrs.lastModifiedTime().toMillis() < MIN_AGE_MILLIS) {
      return false;
    }
    Path lockPath = candidate.resolve(LOCK_NAME);
    if (!Files.exists(lockPath, LinkOption.NOFOLLOW_LINKS)) {
      return true;
    }
    return Files.isRegularFile(lockPath, LinkOption.NOFOLLOW_LINKS) && lockIsFree(lockPath);
  }

  private static boolean lockIsFree(Path lockPath) throws IOException {
    try (FileChannel channel =
        FileChannel.open(lockPath, StandardOpenOption.READ, StandardOpenOption.WRITE)) {
      FileLock lock = channel.tryLock();
      if (lock == null) {
        return false;
      }
      lock.release();
      return true;
    } catch (OverlappingFileLockException e) {
      return false;
    }
  }

  /** Deletes {@code root} and its contents. Symlinks are deleted, never followed. */
  private static void deleteTree(Path root) throws IOException {
    Files.walkFileTree(
        root,
        new SimpleFileVisitor<Path>() {
          @Override
          public FileVisitResult visitFile(Path file, BasicFileAttributes attrs)
              throws IOException {
            Files.delete(file);
            return FileVisitResult.CONTINUE;
          }

          @Override
          public FileVisitResult postVisitDirectory(Path dir, IOException exc) throws IOException {
            if (exc != null) {
              throw exc;
            }
            Files.delete(dir);
            return FileVisitResult.CONTINUE;
          }
        });
  }

  private static void closeQuietly(FileChannel channel) {
    try {
      channel.close();
    } catch (IOException e) {
      logRecoverableFailure("Could not close native-library lock channel", e);
    }
  }

  private static void logRecoverableFailure(String message, Throwable e) {
    logger.warn("{}: {}", message, e.getClass().getName());
  }
}
