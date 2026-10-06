package net.snowflake.client.internal.api.implementation.resultset;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import net.snowflake.client.internal.api.implementation.parameters.ParametersRegistry;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetReleaseResponse;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

class CoreResultSetProviderTest {

  private static final ResultSetHandle HANDLE =
      ResultSetHandle.newBuilder().setId(42).setMagic(99).build();
  private static final String QUERY_ID = "01ab-cdef-0000-0000";

  private CoreDriverApi mockCoreApi;

  @BeforeEach
  void setUp() {
    mockCoreApi = mock(CoreDriverApi.class);
  }

  @Test
  void shouldReleaseHandleOnceWhenReleasedMoreThanOnce() {
    when(mockCoreApi.resultSetRelease(HANDLE))
        .thenReturn(ResultSetReleaseResponse.getDefaultInstance());
    CoreResultSetProvider provider = createProvider();

    provider.release();
    provider.release();

    verify(mockCoreApi, times(1)).resultSetRelease(HANDLE);
  }

  @Test
  void shouldReleaseHandleWhenUnreleasedProviderIsGarbageCollected() throws Exception {
    CountDownLatch released = new CountDownLatch(1);
    when(mockCoreApi.resultSetRelease(HANDLE))
        .thenAnswer(
            invocation -> {
              released.countDown();
              return ResultSetReleaseResponse.getDefaultInstance();
            });

    createProvider();

    long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(10);
    while (released.getCount() > 0 && System.nanoTime() < deadline) {
      System.gc();
      released.await(100, TimeUnit.MILLISECONDS);
    }
    assertEquals(0, released.getCount());
  }

  private CoreResultSetProvider createProvider() {
    return new CoreResultSetProvider(mockCoreApi, HANDLE, QUERY_ID, ParametersRegistry.EMPTY);
  }
}
