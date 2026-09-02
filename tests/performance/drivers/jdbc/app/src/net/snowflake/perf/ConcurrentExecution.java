package net.snowflake.perf;

import java.sql.Connection;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Properties;
import java.util.concurrent.BrokenBarrierException;
import java.util.concurrent.CyclicBarrier;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;

/**
 * Concurrent SELECT bursts: one JDBC connection per worker. Connections are opened and set up
 * before burst timing; wall time covers query and fetch only.
 */
final class ConcurrentExecution {

  private static final int BARRIER_TIMEOUT_S = 120;

  private ConcurrentExecution() {}

  static QueryExecution.FetchTestOutput execute(
      String driverType,
      String url,
      Properties props,
      List<String> setupQueries,
      String sql,
      int warmupIterations,
      int iterations,
      int workerCount)
      throws Exception {
    System.out.println("\n=== Executing Concurrent SELECT Test ===");
    System.out.println("Query: " + sql);
    System.out.println(
        "Workers: " + workerCount + " connections (one statement per connection)");

    System.out.println(
        "Opening " + workerCount + " worker connections (excluded from burst timing)...");
    ExecutorService pool = Executors.newFixedThreadPool(workerCount);
    try {
      try (WorkerConnections workers =
          openWorkerConnections(pool, driverType, url, props, setupQueries, workerCount)) {
        System.out.println("Worker connections ready");

        for (int i = 1; i <= warmupIterations; i++) {
          System.out.println("  Warmup burst " + i + "/" + warmupIterations);
          runBurst(pool, workers.connections, sql);
        }

        ResourceMonitor monitor = new ResourceMonitor(100);
        monitor.start();
        List<QueryExecution.IterationResult> results = new ArrayList<>();
        for (int i = 1; i <= iterations; i++) {
          QueryExecution.IterationResult result = runBurst(pool, workers.connections, sql);
          results.add(result);
          System.out.printf(
              "  Iteration %d/%d: burst=%.3fs  throughput=%.0f rows/s  rows=%d%n",
              i, iterations, result.queryTimeS, result.throughputRowsS, result.rowCount);
        }
        List<ResourceMonitor.Sample> timeline = monitor.stop();

        validateRowCounts(results, workerCount);
        printStatistics(results);
        System.out.println("  Memory timeline: " + timeline.size() + " samples collected");
        return new QueryExecution.FetchTestOutput(results, timeline);
      }
    } finally {
      pool.shutdownNow();
      try {
        pool.awaitTermination(30, TimeUnit.SECONDS);
      } catch (InterruptedException e) {
        Thread.currentThread().interrupt();
      }
    }
  }

  private static WorkerConnections openWorkerConnections(
      ExecutorService pool,
      String driverType,
      String url,
      Properties props,
      List<String> setupQueries,
      int workerCount)
      throws Exception {
    WorkerConnections workers = new WorkerConnections();
    if (!setupQueries.isEmpty()) {
      System.out.println(
          "Running setup queries on " + workerCount + " worker connections...");
    }
    List<Future<Connection>> futures = new ArrayList<>(workerCount);
    for (int i = 0; i < workerCount; i++) {
      futures.add(
          pool.submit(
              () -> {
                Connection conn = ConnectionFactory.connect(driverType, url, props);
                ConnectionFactory.executeSetupQueries(conn, setupQueries, false);
                return conn;
              }));
    }

    Exception firstError = null;
    for (Future<Connection> future : futures) {
      try {
        workers.connections.add(future.get());
      } catch (ExecutionException e) {
        Exception workerError = unwrap(e);
        if (firstError == null) {
          firstError = workerError;
        } else {
          System.err.println("Additional worker connection failure:");
          workerError.printStackTrace(System.err);
        }
      } catch (InterruptedException e) {
        Thread.currentThread().interrupt();
        workers.close();
        throw new IllegalStateException("Worker connection setup interrupted", e);
      }
    }
    if (firstError != null) {
      workers.close();
      throw firstError;
    }
    return workers;
  }

  private static QueryExecution.IterationResult runBurst(
      ExecutorService pool, List<Connection> connections, String sql) throws Exception {
    int workerCount = connections.size();
    long[] burstStartNs = {0L};
    CyclicBarrier barrier =
        new CyclicBarrier(workerCount + 1, () -> burstStartNs[0] = System.nanoTime());

    List<Future<Long>> futures = new ArrayList<>(workerCount);
    for (Connection conn : connections) {
      futures.add(
          pool.submit(
              () -> {
                barrier.await(BARRIER_TIMEOUT_S, TimeUnit.SECONDS);
                return QueryExecution.fetchAllRows(conn, sql);
              }));
    }
    try {
      barrier.await(BARRIER_TIMEOUT_S, TimeUnit.SECONDS);
    } catch (TimeoutException e) {
      throw new IllegalStateException("Concurrent burst barrier timed out", e);
    } catch (BrokenBarrierException e) {
      throw new IllegalStateException("Concurrent burst barrier broken", e);
    } catch (InterruptedException e) {
      Thread.currentThread().interrupt();
      throw new IllegalStateException("Concurrent burst barrier interrupted", e);
    }
    double cpuStart = Common.processCpuSeconds();

    List<Long> workerRows = new ArrayList<>(workerCount);
    Exception firstError = null;
    for (Future<Long> future : futures) {
      try {
        workerRows.add(future.get());
      } catch (ExecutionException e) {
        Exception workerError = unwrap(e);
        if (firstError == null) {
          firstError = workerError;
        } else {
          System.err.println("Additional worker failure:");
          workerError.printStackTrace(System.err);
        }
      }
    }
    if (firstError != null) {
      throw firstError;
    }

    double burstWallS = (System.nanoTime() - burstStartNs[0]) / 1_000_000_000.0;
    long perWorkerRows = workerRows.get(0);
    for (int i = 1; i < workerRows.size(); i++) {
      if (!workerRows.get(i).equals(perWorkerRows)) {
        throw new IllegalStateException(
            "Workers returned unequal row counts: " + workerRows);
      }
    }
    long totalRows = perWorkerRows * workerCount;
    double cpuTimeS = Common.processCpuSeconds() - cpuStart;
    double throughput = burstWallS > 0.0 ? totalRows / burstWallS : 0.0;
    return new QueryExecution.IterationResult(
        System.currentTimeMillis(),
        burstWallS,
        burstWallS,
        totalRows,
        cpuTimeS,
        Common.peakRssMb(),
        workerCount,
        throughput);
  }

  private static Exception unwrap(ExecutionException e) {
    Throwable cause = e.getCause();
    if (cause instanceof Exception) {
      return (Exception) cause;
    }
    return e;
  }

  private static void validateRowCounts(
      List<QueryExecution.IterationResult> results, int workerCount) {
    if (results.isEmpty()) {
      return;
    }
    long expected = results.get(0).rowCount;
    if (expected == 0) {
      throw new IllegalStateException(
          "Row count baseline is 0 — refusing to use 0 as a concurrent-burst baseline.");
    }
    long perWorker = expected / workerCount;
    for (int i = 0; i < results.size(); i++) {
      long actual = results.get(i).rowCount;
      if (actual != expected) {
        throw new IllegalStateException(
            String.format(
                "Row count mismatch: iteration %d returned %d rows, expected %d (%d workers × %d)",
                i, actual, expected, workerCount, perWorker));
      }
    }
    System.out.printf(
        "All %d bursts returned %d rows (%d × %d)%n",
        results.size(), expected, workerCount, perWorker);
  }

  private static void printStatistics(List<QueryExecution.IterationResult> results) {
    if (results.isEmpty()) {
      return;
    }
    List<Double> burstTimes = new ArrayList<>(results.size());
    List<Double> throughputs = new ArrayList<>(results.size());
    for (QueryExecution.IterationResult result : results) {
      burstTimes.add(result.queryTimeS);
      throughputs.add(result.throughputRowsS);
    }
    System.out.println("\nSummary:");
    System.out.printf(
        "  Burst wall: median=%.3fs  min=%.3f  max=%.3f%n",
        median(burstTimes), Collections.min(burstTimes), Collections.max(burstTimes));
    System.out.printf(
        "  Throughput: median=%.0f rows/s  min=%.0f  max=%.0f%n",
        median(throughputs), Collections.min(throughputs), Collections.max(throughputs));
  }

  private static double median(List<Double> values) {
    List<Double> sorted = new ArrayList<>(values);
    Collections.sort(sorted);
    int n = sorted.size();
    if (n % 2 == 1) {
      return sorted.get(n / 2);
    }
    return (sorted.get(n / 2 - 1) + sorted.get(n / 2)) / 2.0;
  }

  private static final class WorkerConnections implements AutoCloseable {
    final List<Connection> connections = new ArrayList<>();

    @Override
    public void close() {
      for (int i = connections.size() - 1; i >= 0; i--) {
        try {
          connections.get(i).close();
        } catch (SQLException ignored) {
          // ignore
        }
      }
    }
  }
}
