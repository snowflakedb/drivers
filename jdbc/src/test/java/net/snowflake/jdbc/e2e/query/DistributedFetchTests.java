package net.snowflake.jdbc.e2e.query;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.ObjectInputStream;
import java.io.ObjectOutputStream;
import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.TimeUnit;
import net.snowflake.client.api.resultset.SnowflakeResultSet;
import net.snowflake.client.api.resultset.SnowflakeResultSetSerializable;
import net.snowflake.client.api.resultset.SnowflakeResultSetSerializable.ResultSetRetrieveConfig;
import net.snowflake.jdbc.utils.SkipOldDriver;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

/**
 * Distributed-fetch tests: split a result set into independently serializable partitions, serialize
 * each one on its own, then in parallel worker threads deserialize a partition and fetch its rows
 * without a live session.
 */
class DistributedFetchTests extends SnowflakeIntegrationTestBase {

  private static final int LARGE_RESULT_SET_ROW_COUNT = 100_000;

  private static final String LARGE_RESULT_SET_QUERY =
      "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => " + LARGE_RESULT_SET_ROW_COUNT + ")) v";

  // Split so each result chunk becomes its own serializable partition (inline + one per remote
  // chunk).
  private static final long SPLIT_PER_CHUNK = 1L;

  // getResultSet ignores the URL today (the retrieval reuses the process-wide core API), but the
  // config builder still requires a protocol-qualified URL, so pass a valid placeholder.
  private static final String PLACEHOLDER_URL = "https://snowflake.snowflakecomputing.com";

  @Test
  @SkipOldDriver("fix in the old driver not yet released")
  void shouldFetchAllRowsWhenPartitionsFetchedInParallelThreads() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    List<byte[]> serializedSlices;
    // And Query "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 100000)) v" is executed
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(LARGE_RESULT_SET_QUERY)) {

      // And the result set is split into independently serializable partitions
      List<SnowflakeResultSetSerializable> slices =
          resultSet.unwrap(SnowflakeResultSet.class).getResultSetSerializables(SPLIT_PER_CHUNK);

      // Then there should be at least two partitions
      assertTrue(slices.size() >= 2, "Expected at least an inline slice and one remote slice");

      // When each partition is serialized and fetched on its own worker thread without a live
      // session
      serializedSlices = new ArrayList<>(slices.size());
      for (SnowflakeResultSetSerializable slice : slices) {
        serializedSlices.add(serialize(slice));
      }
    }

    // Each slice is fetched independently on a worker thread; ids are aggregated then sorted, so
    // the (non-deterministic) cross-thread arrival order does not affect the assertions.
    List<Long> allIds = fetchSlicesInParallel(serializedSlices);

    // Then the combined row count across all threads should be 100000
    assertEquals(LARGE_RESULT_SET_ROW_COUNT, allIds.size());

    // And all ids from 0 to 99999 should be present exactly once
    Collections.sort(allIds);
    assertEquals(sequentialIds(LARGE_RESULT_SET_ROW_COUNT), allIds);
  }

  @Test
  void shouldPreserveRowCountAndDataSizesAcrossPartitionSplit() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 100000)) v" is executed
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(LARGE_RESULT_SET_QUERY)) {
      SnowflakeResultSet snowflakeResultSet = resultSet.unwrap(SnowflakeResultSet.class);

      // A single slice holding every chunk is the authoritative metadata baseline.
      SnowflakeResultSetSerializable wholeSlice = singleUnboundedSlice(snowflakeResultSet);
      long expectedRowCount = wholeSlice.getRowCount();
      long expectedCompressedSize = wholeSlice.getCompressedDataSizeInBytes();
      long expectedUncompressedSize = wholeSlice.getUncompressedDataSizeInBytes();
      assertTrue(expectedCompressedSize > 0, "Expected positive compressed size");
      assertTrue(expectedUncompressedSize > 0, "Expected positive uncompressed size");

      // And the result set is split into independently serializable partitions
      List<SnowflakeResultSetSerializable> slices =
          snowflakeResultSet.getResultSetSerializables(SPLIT_PER_CHUNK);
      long actualRowCount = 0;
      long actualCompressedSize = 0;
      long actualUncompressedSize = 0;
      for (SnowflakeResultSetSerializable slice : slices) {
        actualRowCount += slice.getRowCount();
        actualCompressedSize += slice.getCompressedDataSizeInBytes();
        actualUncompressedSize += slice.getUncompressedDataSizeInBytes();
      }

      // Then the sum of the partition row counts should be 100000
      assertEquals(LARGE_RESULT_SET_ROW_COUNT, actualRowCount);
      assertEquals(expectedRowCount, actualRowCount);

      // And the aggregate compressed and uncompressed data sizes should be preserved across the
      // split
      assertEquals(expectedCompressedSize, actualCompressedSize);
      assertEquals(expectedUncompressedSize, actualUncompressedSize);
    }
  }

  @Test
  @SkipOldDriver("fix in the old driver not yet released")
  void shouldRoundTripResultSetThroughSerializableRepeatedly() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When Query "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 100000)) v" is executed
    try (Statement statement = connection.createStatement();
        ResultSet resultSet = statement.executeQuery(LARGE_RESULT_SET_QUERY)) {

      // And the result set is round tripped through a serializable and back to a result set
      SnowflakeResultSetSerializable firstSlice =
          singleUnboundedSlice(resultSet.unwrap(SnowflakeResultSet.class));
      try (ResultSet rehydrated =
          deserialize(serialize(firstSlice)).getResultSet(retrieveConfig())) {

        // And the rehydrated result set is round tripped through a serializable a second time
        SnowflakeResultSetSerializable secondSlice =
            singleUnboundedSlice(rehydrated.unwrap(SnowflakeResultSet.class));

        try (ResultSet finalResultSet =
            deserialize(serialize(secondSlice)).getResultSet(retrieveConfig())) {
          // Then the twice round tripped result set should expose all 100000 rows
          List<Long> ids = new ArrayList<>();
          while (finalResultSet.next()) {
            ids.add(finalResultSet.getLong(1));
          }
          assertEquals(LARGE_RESULT_SET_ROW_COUNT, ids.size());

          // And all ids from 0 to 99999 should be present exactly once
          Collections.sort(ids);
          assertEquals(sequentialIds(LARGE_RESULT_SET_ROW_COUNT), ids);
        }
      }
    }
  }

  @Test
  @SkipOldDriver("fix in the old driver not yet released")
  void shouldPreserveSessionTimezoneForTimestampLtzFetchedFromSerializableWithoutALiveSession()
      throws Exception {
    // Pick a non-default session TIMEZONE and TIMESTAMP_LTZ_OUTPUT_FORMAT so that a serializable
    // which drops the session's DataConversionContext renders getString() differently from the
    // live result set (both format and timezone offset diverge).
    // Given Snowflake client is logged in with a non-default session timezone
    String expectedLiveValue;
    byte[] serialized;
    try (Connection connection = openConnection();
        Statement statement = connection.createStatement()) {
      statement.execute("alter session set TIMEZONE = 'Europe/Warsaw'");
      statement.execute(
          "alter session set TIMESTAMP_LTZ_OUTPUT_FORMAT = 'YYYY-MM-DD HH24:MI:SS.FF3 TZH:TZM'");

      // When a query returning TIMESTAMP_LTZ values is executed
      try (ResultSet resultSet =
          statement.executeQuery("SELECT '2014-01-11 06:12:13.123456789'::timestamp_ltz AS ts")) {
        // Slice the live core handle before consuming the cursor; the split reads chunk metadata
        // independently of the Arrow stream, so the live read below is unaffected.
        // And the result set is split into independently serializable partitions
        serialized = serialize(singleUnboundedSlice(resultSet.unwrap(SnowflakeResultSet.class)));

        assertTrue(resultSet.next(), "Expected one row from the live result set");
        expectedLiveValue = resultSet.getString(1);
      }
    }

    // And each partition is serialized and fetched without a live session
    try (ResultSet rehydrated = deserialize(serialized).getResultSet(retrieveConfig())) {
      assertTrue(rehydrated.next(), "Expected one row from the rehydrated result set");
      String rehydratedValue = rehydrated.getString(1);

      // A serializable-derived ResultSet must format TIMESTAMP_LTZ with the originating session's
      // TIMESTAMP_LTZ_OUTPUT_FORMAT and TIMEZONE. Fails today: the sessionless conversion context
      // falls back to interface defaults, losing both the format string and the session timezone.
      // Then the fetched timestamp values should match those rendered by the originating session
      assertEquals(expectedLiveValue, rehydratedValue);
    }
  }

  private static List<Long> fetchSlicesInParallel(List<byte[]> serializedSlices) throws Exception {
    ExecutorService pool = Executors.newFixedThreadPool(4);
    try {
      List<Future<List<Long>>> futures = new ArrayList<>(serializedSlices.size());
      for (byte[] serializedSlice : serializedSlices) {
        futures.add(pool.submit(() -> fetchSliceIds(serializedSlice)));
      }
      List<Long> allIds = new ArrayList<>();
      for (Future<List<Long>> future : futures) {
        allIds.addAll(future.get(120, TimeUnit.SECONDS));
      }
      return allIds;
    } finally {
      pool.shutdown();
      assertTrue(
          pool.awaitTermination(30, TimeUnit.SECONDS), "Fetch thread pool did not terminate");
    }
  }

  private static List<Long> fetchSliceIds(byte[] serializedSlice) throws Exception {
    SnowflakeResultSetSerializable slice = deserialize(serializedSlice);
    List<Long> ids = new ArrayList<>();
    try (ResultSet resultSet = slice.getResultSet(retrieveConfig())) {
      while (resultSet.next()) {
        ids.add(resultSet.getLong(1));
      }
    }
    return ids;
  }

  /**
   * Splits with an unbounded max size so the whole result collapses into one serializable slice.
   */
  private static SnowflakeResultSetSerializable singleUnboundedSlice(SnowflakeResultSet resultSet)
      throws SQLException {
    List<SnowflakeResultSetSerializable> slices =
        resultSet.getResultSetSerializables(Long.MAX_VALUE);
    assertEquals(1, slices.size(), "Expected a single slice for an unbounded split");
    return slices.get(0);
  }

  private static ResultSetRetrieveConfig retrieveConfig() {
    return ResultSetRetrieveConfig.Builder.newInstance().setSfFullURL(PLACEHOLDER_URL).build();
  }

  private static List<Long> sequentialIds(int count) {
    List<Long> ids = new ArrayList<>(count);
    for (long i = 0; i < count; i++) {
      ids.add(i);
    }
    return ids;
  }

  private static byte[] serialize(SnowflakeResultSetSerializable slice) throws IOException {
    ByteArrayOutputStream bytes = new ByteArrayOutputStream();
    try (ObjectOutputStream out = new ObjectOutputStream(bytes)) {
      out.writeObject(slice);
    }
    return bytes.toByteArray();
  }

  private static SnowflakeResultSetSerializable deserialize(byte[] serialized)
      throws IOException, ClassNotFoundException {
    try (ObjectInputStream in = new ObjectInputStream(new ByteArrayInputStream(serialized))) {
      return (SnowflakeResultSetSerializable) in.readObject();
    }
  }
}
