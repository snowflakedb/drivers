package net.snowflake.client.internal.unicore;

import java.util.concurrent.ExecutionException;
import java.util.concurrent.Future;
import lombok.AccessLevel;
import lombok.NoArgsConstructor;
import net.snowflake.client.internal.api.implementation.exception.CoreException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DriverException;

/** Blocks on core RPC futures and maps their failures to {@link CoreException} carriers. */
@NoArgsConstructor(access = AccessLevel.PRIVATE)
public final class CoreFutures {

  /**
   * Block on an async-first RPC future, mapping its failure to a {@link CoreException} runtime
   * carrier. {@link CoreFuture#get()} wraps the decoder's {@code ServiceException} / {@code
   * TransportException} in an {@link ExecutionException}, so unwrap the cause here.
   */
  public static <T> T await(Future<T> future) {
    try {
      return future.get();
    } catch (ExecutionException e) {
      Throwable cause = e.getCause();
      if (cause instanceof ServiceException || cause instanceof TransportException) {
        throw toCoreException((RuntimeException) cause);
      }
      throw new CoreException(
          "Driver error: " + (cause != null ? cause.getMessage() : "unknown"),
          cause != null ? cause : e);
    } catch (InterruptedException e) {
      Thread.currentThread().interrupt();
      throw new CoreException("Interrupted while waiting for the core operation to complete", e);
    }
  }

  /**
   * Map an RPC failure — thrown directly by a blocking call or unwrapped from an {@link
   * ExecutionException} on the async path — to a {@link CoreException} runtime carrier. Any {@link
   * DriverException} payload is preserved verbatim so the decorator boundary can reconstruct the
   * core-reported SQLState / vendor code / query id.
   */
  static CoreException toCoreException(RuntimeException e) {
    if (e instanceof ServiceException) {
      Object error = ((ServiceException) e).error;
      if (error instanceof DriverException) {
        return new CoreException((DriverException) error, e);
      }
      return new CoreException(e.getMessage(), e);
    }
    // TransportException
    return new CoreException("Driver communication error: " + e.getMessage(), e);
  }
}
