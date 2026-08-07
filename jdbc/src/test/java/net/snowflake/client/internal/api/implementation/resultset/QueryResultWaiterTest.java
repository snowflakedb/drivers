package net.snowflake.client.internal.api.implementation.resultset;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.UUID;
import java.util.concurrent.atomic.AtomicInteger;
import net.snowflake.client.api.resultset.QueryStatus;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import org.junit.jupiter.api.Test;

class QueryResultWaiterTest {

  private final String queryId = UUID.randomUUID().toString();

  // No-op backoff so polling tests don't sleep through real wall-clock retries.
  // (The NO_DATA path alone backs off ~124s.) Tests that exercise the real
  // Thread.sleep interrupt path use the default two-arg constructor instead.
  private static final QueryResultWaiter.Sleeper NO_SLEEP = millis -> {};

  private static QueryStatus status(String name) {
    return new QueryStatus(0, 0, "", "", name, 0, "", 0, name, 0, "", 0, "", "");
  }

  private static QueryStatus errorStatus(String name, int errorCode, String errorMessage) {
    return new QueryStatus(0, errorCode, errorMessage, "", name, 0, "", 0, name, 0, "", 0, "", "");
  }

  @Test
  void shouldReturnImmediatelyOnSuccess() throws Exception {
    QueryStatus success = status("SUCCESS");
    QueryResultWaiter waiter = new QueryResultWaiter(() -> success, queryId);

    QueryStatus result = waiter.waitForCompletion();

    assertSame(success, result);
  }

  @Test
  void shouldPollUntilSuccess() throws Exception {
    AtomicInteger calls = new AtomicInteger();
    QueryResultWaiter waiter =
        new QueryResultWaiter(
            () -> {
              if (calls.incrementAndGet() < 3) {
                return status("RUNNING");
              }
              return status("SUCCESS");
            },
            queryId,
            NO_SLEEP);

    QueryStatus result = waiter.waitForCompletion();

    assertEquals("SUCCESS", result.getName());
    assertEquals(3, calls.get());
  }

  @Test
  void shouldThrowOnFailedWithError() {
    QueryStatus failed = errorStatus("FAILED_WITH_ERROR", 100123, "Query compilation error");
    QueryResultWaiter waiter = new QueryResultWaiter(() -> failed, queryId);

    SFSQLException thrown = assertThrows(SFSQLException.class, waiter::waitForCompletion);

    assertTrue(thrown.getMessage().contains("FAILED_WITH_ERROR"));
    assertTrue(thrown.getMessage().contains("Query compilation error"));
    assertEquals(queryId, thrown.getQueryId());
  }

  @Test
  void shouldThrowOnAborted() {
    QueryStatus aborted = errorStatus("ABORTED", 0, "");
    QueryResultWaiter waiter = new QueryResultWaiter(() -> aborted, queryId);

    SFSQLException thrown = assertThrows(SFSQLException.class, waiter::waitForCompletion);

    assertTrue(thrown.getMessage().contains("ABORTED"));
    assertTrue(thrown.getMessage().contains("No error message available"));
    assertEquals(queryId, thrown.getQueryId());
  }

  @Test
  void shouldThrowAfterMaxNoDataRetries() {
    QueryResultWaiter waiter = new QueryResultWaiter(() -> status("NO_DATA"), queryId, NO_SLEEP);

    SFSQLException thrown = assertThrows(SFSQLException.class, waiter::waitForCompletion);

    assertTrue(thrown.getMessage().contains("Cannot retrieve data"));
    assertTrue(thrown.getMessage().contains(queryId));
    assertEquals(queryId, thrown.getQueryId());
  }

  @Test
  void shouldPollThroughMultipleStatusTransitions() throws Exception {
    AtomicInteger calls = new AtomicInteger();
    QueryResultWaiter waiter =
        new QueryResultWaiter(
            () -> {
              int n = calls.incrementAndGet();
              if (n <= 2) {
                return status("QUEUED");
              }
              if (n <= 4) {
                return status("RESUMING_WAREHOUSE");
              }
              if (n <= 5) {
                return status("RUNNING");
              }
              return status("SUCCESS");
            },
            queryId,
            NO_SLEEP);

    QueryStatus result = waiter.waitForCompletion();

    assertEquals("SUCCESS", result.getName());
    assertEquals(6, calls.get());
  }

  @Test
  void shouldPropagateExceptionFromStatusCheck() {
    SFSQLException apiError = new SFSQLException("connection lost");
    QueryResultWaiter waiter =
        new QueryResultWaiter(
            () -> {
              throw apiError;
            },
            queryId);

    SFSQLException thrown = assertThrows(SFSQLException.class, waiter::waitForCompletion);

    assertSame(apiError, thrown);
  }

  @Test
  void shouldHandleInterruptDuringWait() {
    QueryResultWaiter waiter = new QueryResultWaiter(() -> status("RUNNING"), queryId);

    Thread.currentThread().interrupt();
    SFSQLException thrown = assertThrows(SFSQLException.class, waiter::waitForCompletion);

    assertTrue(thrown.getMessage().contains("Interrupted"));
    assertEquals(queryId, thrown.getQueryId());
    assertTrue(Thread.interrupted(), "Thread interrupt flag should be re-set");
  }
}
