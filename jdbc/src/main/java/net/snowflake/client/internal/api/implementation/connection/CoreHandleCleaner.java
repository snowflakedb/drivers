package net.snowflake.client.internal.api.implementation.connection;

import java.lang.ref.PhantomReference;
import java.lang.ref.ReferenceQueue;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;

/**
 * Runs a release action for core handles once, either when {@link Cleanable#clean()} is called or
 * after the owner becomes phantom-reachable. A Java 8 stand-in for {@code java.lang.ref.Cleaner}.
 *
 * <p>The action must not reference the owner, otherwise the owner never becomes unreachable.
 */
class CoreHandleCleaner {

  private static final SFLogger logger = SFLoggerFactory.getLogger(CoreHandleCleaner.class);

  private static final ReferenceQueue<Object> QUEUE = new ReferenceQueue<>();
  private static final Set<Cleanable> PENDING = ConcurrentHashMap.newKeySet();

  static {
    // Single process-wide daemon; it pins the driver's classloader. Actions run inline here, so a
    // blocking action delays every later release.
    Thread thread = new Thread(CoreHandleCleaner::drain, "snowflake-core-handle-cleaner");
    thread.setDaemon(true);
    thread.start();
  }

  private CoreHandleCleaner() {}

  static Cleanable register(Object owner, Runnable action) {
    Cleanable cleanable = new Cleanable(owner, action);
    PENDING.add(cleanable);
    return cleanable;
  }

  private static void drain() {
    while (true) {
      try {
        ((Cleanable) QUEUE.remove()).clean();
      } catch (InterruptedException e) {
        logger.debug("Core handle cleaner interrupted, continuing", e);
      } catch (Throwable e) {
        logger.debug("Error releasing native handles of an unreachable object", e);
      }
    }
  }

  static final class Cleanable extends PhantomReference<Object> {
    private final Runnable action;

    private Cleanable(Object owner, Runnable action) {
      super(owner, QUEUE);
      this.action = action;
    }

    void clean() {
      if (PENDING.remove(this)) {
        action.run();
      }
    }
  }
}
