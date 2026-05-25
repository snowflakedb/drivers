package net.snowflake.client.internal.api.implementation.resultset;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.SQLException;
import java.util.UUID;
import java.util.concurrent.atomic.AtomicInteger;
import net.snowflake.client.api.resultset.QueryStatus;
import org.junit.jupiter.api.Test;

class QueryResultWaiterTest {

  private final String queryId = UUID.randomUUID().toString();

  private static QueryStatus status(String name) {
    return new QueryStatus(0, 0, "", "", name, 0, "", 0, name, 0, "", 0, "", "");
  }

  private static QueryStatus errorStatus(String name, int errorCode, String errorMessage) {
    return new QueryStatus(0, errorCode, errorMessage, "", name, 0, "", 0, name, 0, "", 0, "", "");
  }

  @Test
  void returnsImmediatelyOnSuccess() throws Exception {
    QueryStatus success = status("SUCCESS");
    QueryResultWaiter waiter = new QueryResultWaiter(() -> success, queryId);

    QueryStatus result = waiter.waitForCompletion();

    assertSame(success, result);
  }

  @Test
  void pollsUntilSuccess() throws Exception {
    AtomicInteger calls = new AtomicInteger();
    QueryResultWaiter waiter =
        new QueryResultWaiter(
            () -> {
              if (calls.incrementAndGet() < 3) {
                return status("RUNNING");
              }
              return status("SUCCESS");
            },
            queryId);

    QueryStatus result = waiter.waitForCompletion();

    assertEquals("SUCCESS", result.getName());
    assertEquals(3, calls.get());
  }

  @Test
  void throwsOnFailedWithError() {
    QueryStatus failed = errorStatus("FAILED_WITH_ERROR", 100123, "Query compilation error");
    QueryResultWaiter waiter = new QueryResultWaiter(() -> failed, queryId);

    SQLException thrown = assertThrows(SQLException.class, waiter::waitForCompletion);

    assertTrue(thrown.getMessage().contains("FAILED_WITH_ERROR"));
    assertTrue(thrown.getMessage().contains("Query compilation error"));
  }

  @Test
  void throwsOnAborted() {
    QueryStatus aborted = errorStatus("ABORTED", 0, "");
    QueryResultWaiter waiter = new QueryResultWaiter(() -> aborted, queryId);

    SQLException thrown = assertThrows(SQLException.class, waiter::waitForCompletion);

    assertTrue(thrown.getMessage().contains("ABORTED"));
    assertTrue(thrown.getMessage().contains("No error message available"));
  }

  @Test
  void throwsAfterMaxNoDataRetries() {
    QueryResultWaiter waiter = new QueryResultWaiter(() -> status("NO_DATA"), queryId);

    SQLException thrown = assertThrows(SQLException.class, waiter::waitForCompletion);

    assertTrue(thrown.getMessage().contains("Cannot retrieve data"));
    assertTrue(thrown.getMessage().contains(queryId));
  }

  @Test
  void pollsThroughMultipleStatusTransitions() throws Exception {
    AtomicInteger calls = new AtomicInteger();
    QueryResultWaiter waiter =
        new QueryResultWaiter(
            () -> {
              int n = calls.incrementAndGet();
              if (n <= 2) return status("QUEUED");
              if (n <= 4) return status("RESUMING_WAREHOUSE");
              if (n <= 5) return status("RUNNING");
              return status("SUCCESS");
            },
            queryId);

    QueryStatus result = waiter.waitForCompletion();

    assertEquals("SUCCESS", result.getName());
    assertEquals(6, calls.get());
  }

  @Test
  void propagatesExceptionFromStatusCheck() {
    SQLException apiError = new SQLException("connection lost");
    QueryResultWaiter waiter =
        new QueryResultWaiter(
            () -> {
              throw apiError;
            },
            queryId);

    SQLException thrown = assertThrows(SQLException.class, waiter::waitForCompletion);

    assertSame(apiError, thrown);
  }

  @Test
  void handlesInterruptDuringWait() {
    QueryResultWaiter waiter = new QueryResultWaiter(() -> status("RUNNING"), queryId);

    Thread.currentThread().interrupt();
    SQLException thrown = assertThrows(SQLException.class, waiter::waitForCompletion);

    assertTrue(thrown.getMessage().contains("Interrupted"));
    assertTrue(Thread.interrupted(), "Thread interrupt flag should be re-set");
  }
}
