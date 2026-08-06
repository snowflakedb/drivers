package net.snowflake.client.internal.unicore;

import java.util.concurrent.ExecutionException;
import java.util.concurrent.Future;
import java.util.concurrent.TimeUnit;
import lombok.RequiredArgsConstructor;

/**
 * A {@link Future} over an async-first core RPC submitted through {@link
 * CoreTransport#submitMessage}.
 *
 * <p><b>Lazy-await:</b> the RPC runs in the background from the moment it is submitted; {@link
 * #get()} runs the blocking {@link CoreTransport#awaitMessage} on the calling thread and decodes
 * the response. Cancelling from another thread ({@link #cancel(boolean)}) flips the core
 * cancellation token, which completes the background operation and unblocks a parked {@code get()}.
 *
 * <p>{@link #get(long, TimeUnit)} and pre-{@code get()} completion state are intentionally not
 * supported yet — they require an executor-backed variant, which will be added when a timeout or
 * cancel trigger (e.g. JDBC login timeout) is wired.
 *
 * @param <T> decoded response type
 */
@RequiredArgsConstructor
public final class CoreFuture<T> implements Future<T> {

  /** Decodes a raw {@link CoreTransport.TransportResponse} into the typed response. */
  @FunctionalInterface
  public interface Decoder<T> {
    T decode(CoreTransport.TransportResponse response) throws TransportException;
  }

  private final CoreTransport transport;
  private final long handle;
  private final Decoder<T> decoder;

  private volatile boolean cancelled = false;
  private volatile boolean done = false;
  private T result;
  private Throwable failure;

  @Override
  public synchronized T get() throws InterruptedException, ExecutionException {
    if (!done) {
      try {
        result = decoder.decode(transport.awaitMessage(handle));
      } catch (Throwable t) {
        failure = t;
      }
      done = true;
    }
    if (failure != null) {
      throw new ExecutionException(failure);
    }
    return result;
  }

  @Override
  public T get(long timeout, TimeUnit unit) {
    throw new UnsupportedOperationException(
        "Timed get() is not supported yet; needs an executor-backed CoreFuture or a"
            + " timeout-aware awaitMessage");
  }

  @Override
  public boolean cancel(boolean mayInterruptIfRunning) {
    // Must NOT synchronize on this: a parked get() holds the monitor while blocked in
    // awaitMessage, so locking here would deadlock. Just flip the core token; the background
    // operation then resolves to a Cancelled result and the parked get() returns.
    cancelled = true;
    transport.cancel(handle);
    return true;
  }

  @Override
  public boolean isCancelled() {
    return cancelled;
  }

  @Override
  public boolean isDone() {
    return done || cancelled;
  }
}
