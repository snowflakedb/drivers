package net.snowflake.client.internal.api.implementation.resultset;

import java.sql.SQLException;
import net.snowflake.client.api.resultset.QueryStatus;

/** Polls query status with capped exponential backoff until the query completes or errors. */
class QueryResultWaiter {

  private static final int[] RETRY_PATTERN = {1, 1, 2, 3, 4, 8, 10};
  private static final int NO_DATA_MAX_RETRIES = 30;

  private final QueryStatusCheck statusCheck;
  private final String queryId;
  private final Sleeper sleeper;

  @FunctionalInterface
  interface QueryStatusCheck {
    QueryStatus get() throws SQLException;
  }

  /**
   * Backoff sleep seam. Production uses {@link Thread#sleep(long)}; tests inject a no-op so the
   * polling paths don't sleep through real wall-clock backoff (the NO_DATA path alone is ~124s).
   */
  @FunctionalInterface
  interface Sleeper {
    void sleep(long millis) throws InterruptedException;
  }

  QueryResultWaiter(QueryStatusCheck statusCheck, String queryId) {
    this(statusCheck, queryId, Thread::sleep);
  }

  QueryResultWaiter(QueryStatusCheck statusCheck, String queryId, Sleeper sleeper) {
    this.statusCheck = statusCheck;
    this.queryId = queryId;
    this.sleeper = sleeper;
  }

  /**
   * Block until the query reaches a terminal state.
   *
   * @return the successful {@link QueryStatus}
   * @throws SQLException if the query fails, is aborted, or status cannot be determined
   */
  QueryStatus waitForCompletion() throws SQLException {
    int noDataRetry = 0;
    int retryIdx = 0;
    while (true) {
      QueryStatus status = statusCheck.get();
      if (status.isSuccess()) {
        return status;
      }
      if (!status.isStillRunning()) {
        String errorMessage = status.getErrorMessage();
        if (errorMessage == null || errorMessage.isEmpty()) {
          errorMessage = "No error message available";
        }
        throw new SQLException(
            String.format(
                "Status of query associated with resultSet is %s. %s Results not generated.",
                status.getDescription(), errorMessage));
      }
      if (status.getStatus() == QueryStatus.Status.NO_DATA) {
        noDataRetry++;
        if (noDataRetry >= NO_DATA_MAX_RETRIES) {
          throw new SQLException(
              String.format(
                  "Cannot retrieve data on the status of this query."
                      + " No information returned from server for queryID=%s",
                  queryId));
        }
      }
      try {
        sleeper.sleep(500L * RETRY_PATTERN[retryIdx]);
      } catch (InterruptedException e) {
        Thread.currentThread().interrupt();
        throw new SQLException("Interrupted while waiting for async query to complete", e);
      }
      if (retryIdx < RETRY_PATTERN.length - 1) {
        retryIdx++;
      }
    }
  }
}
