package net.snowflake.client.internal.unicore;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;
import org.junit.jupiter.api.Test;

class CoreFutureTest {

  /** Scriptable {@link CoreTransport} that records the awaited/cancelled handle. */
  private static final class FakeCoreTransport implements CoreTransport {
    private final CoreTransport.TransportResponse awaitResponse;
    private long cancelledHandle = -1;

    FakeCoreTransport(CoreTransport.TransportResponse awaitResponse) {
      this.awaitResponse = awaitResponse;
    }

    @Override
    public CoreTransport.TransportResponse handleMessage(
        String service, String method, byte[] request) {
      throw new UnsupportedOperationException("not used");
    }

    @Override
    public long submitMessage(String service, String method, byte[] request) {
      return 1L;
    }

    @Override
    public CoreTransport.TransportResponse awaitMessage(long handle) {
      return awaitResponse;
    }

    @Override
    public void cancel(long handle) {
      this.cancelledHandle = handle;
    }
  }

  @Test
  void shouldReturnDecodedResponseFromGet() throws Exception {
    CoreTransport.TransportResponse response =
        new CoreTransport.TransportResponse(CoreTransport.CODE_SUCCESS, new byte[] {1, 2, 3});
    FakeCoreTransport transport = new FakeCoreTransport(response);
    CoreFuture<String> future = new CoreFuture<>(transport, 7L, r -> "decoded:" + r.getCode());

    assertEquals("decoded:0", future.get());
    assertTrue(future.isDone());
    assertFalse(future.isCancelled());
  }

  @Test
  void shouldWrapDecoderFailureInExecutionException() {
    CoreTransport.TransportResponse response =
        new CoreTransport.TransportResponse(CoreTransport.CODE_TRANSPORT_ERROR, new byte[0]);
    FakeCoreTransport transport = new FakeCoreTransport(response);
    CoreFuture<String> future =
        new CoreFuture<>(
            transport,
            7L,
            r -> {
              throw new TransportException("boom");
            });

    ExecutionException ex = assertThrows(ExecutionException.class, future::get);
    assertInstanceOf(TransportException.class, ex.getCause());
  }

  @Test
  void shouldCallTransportCancelWhenCancelled() {
    FakeCoreTransport transport =
        new FakeCoreTransport(
            new CoreTransport.TransportResponse(CoreTransport.CODE_SUCCESS, new byte[0]));
    CoreFuture<String> future = new CoreFuture<>(transport, 42L, r -> "ignored");

    assertTrue(future.cancel(true));
    assertEquals(42L, transport.cancelledHandle);
    assertTrue(future.isCancelled());
    assertTrue(future.isDone());
  }

  @Test
  void shouldRejectTimedGetUntilExecutorBacked() {
    FakeCoreTransport transport =
        new FakeCoreTransport(
            new CoreTransport.TransportResponse(CoreTransport.CODE_SUCCESS, new byte[0]));
    CoreFuture<String> future = new CoreFuture<>(transport, 1L, r -> "ignored");

    assertThrows(UnsupportedOperationException.class, () -> future.get(1, TimeUnit.SECONDS));
  }
}
