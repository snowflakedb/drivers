package net.snowflake.client.internal.api.implementation.resultset;

import java.sql.SQLException;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.api.resultset.QueryStatus;

/** Polls query status with capped exponential backoff until the query completes or errors. */
@RequiredArgsConstructor
class QueryResultWaiter {

  private static final int[] RETRY_PATTERN = {1, 1, 2, 3, 4, 8, 10};
  private static final int NO_DATA_MAX_RETRIES = 30;

  private final QueryStatusCheck statusCheck;
  private final String queryId;

  @FunctionalInterface
  interface QueryStatusCheck {
    QueryStatus get() throws SQLException;
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
        Thread.sleep(500L * RETRY_PATTERN[retryIdx]);
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
