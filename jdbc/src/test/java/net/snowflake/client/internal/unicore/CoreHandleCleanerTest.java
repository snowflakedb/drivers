package net.snowflake.client.internal.unicore;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.Test;

class CoreHandleCleanerTest {

  @Test
  void shouldRunActionOnceWhenCleanedExplicitlyMoreThanOnce() {
    AtomicInteger runs = new AtomicInteger();
    CoreHandleCleaner.Cleanable cleanable =
        CoreHandleCleaner.register(new Object(), runs::incrementAndGet);

    cleanable.clean();
    cleanable.clean();

    assertEquals(1, runs.get());
  }

  @Test
  void shouldRunActionAfterOwnerIsGarbageCollected() throws InterruptedException {
    CountDownLatch released = new CountDownLatch(1);
    CoreHandleCleaner.register(new Object(), released::countDown);

    long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(10);
    while (released.getCount() > 0 && System.nanoTime() < deadline) {
      System.gc();
      released.await(100, TimeUnit.MILLISECONDS);
    }

    assertEquals(0, released.getCount());
  }
}
