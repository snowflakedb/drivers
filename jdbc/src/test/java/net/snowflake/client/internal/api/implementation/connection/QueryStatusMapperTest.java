package net.snowflake.client.internal.api.implementation.connection;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import net.snowflake.client.api.resultset.QueryStatus;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetQueryStatusResponse;
import org.junit.jupiter.api.Test;

class QueryStatusMapperTest {

  @Test
  void mapsSuccessStatus() {
    ConnectionGetQueryStatusResponse response =
        ConnectionGetQueryStatusResponse.newBuilder().setStatusName("SUCCESS").build();

    QueryStatus status = QueryStatusMapper.fromCoreResponse(response);

    assertEquals("SUCCESS", status.getName());
    assertTrue(status.isSuccess());
    assertFalse(status.isStillRunning());
    assertFalse(status.isAnError());
    assertEquals(0, status.getErrorCode());
    assertEquals("", status.getErrorMessage());
  }

  @Test
  void mapsRunningStatus() {
    ConnectionGetQueryStatusResponse response =
        ConnectionGetQueryStatusResponse.newBuilder().setStatusName("RUNNING").build();

    QueryStatus status = QueryStatusMapper.fromCoreResponse(response);

    assertEquals("RUNNING", status.getName());
    assertFalse(status.isSuccess());
    assertTrue(status.isStillRunning());
    assertFalse(status.isAnError());
  }

  @Test
  void mapsFailedWithErrorAndErrorFields() {
    ConnectionGetQueryStatusResponse response =
        ConnectionGetQueryStatusResponse.newBuilder()
            .setStatusName("FAILED_WITH_ERROR")
            .setErrorCode(100123)
            .setErrorMessage("Compilation error")
            .build();

    QueryStatus status = QueryStatusMapper.fromCoreResponse(response);

    assertEquals("FAILED_WITH_ERROR", status.getName());
    assertFalse(status.isSuccess());
    assertFalse(status.isStillRunning());
    assertTrue(status.isAnError());
    assertEquals(100123, status.getErrorCode());
    assertEquals("Compilation error", status.getErrorMessage());
  }

  @Test
  void mapsQueuedStatus() {
    ConnectionGetQueryStatusResponse response =
        ConnectionGetQueryStatusResponse.newBuilder().setStatusName("QUEUED").build();

    QueryStatus status = QueryStatusMapper.fromCoreResponse(response);

    assertTrue(status.isStillRunning());
    assertFalse(status.isSuccess());
  }

  @Test
  void mapsAbortedStatus() {
    ConnectionGetQueryStatusResponse response =
        ConnectionGetQueryStatusResponse.newBuilder().setStatusName("ABORTED").build();

    QueryStatus status = QueryStatusMapper.fromCoreResponse(response);

    assertTrue(status.isAnError());
    assertFalse(status.isStillRunning());
    assertFalse(status.isSuccess());
  }

  @Test
  void defaultsErrorFieldsWhenAbsent() {
    ConnectionGetQueryStatusResponse response =
        ConnectionGetQueryStatusResponse.newBuilder().setStatusName("RUNNING").build();

    QueryStatus status = QueryStatusMapper.fromCoreResponse(response);

    assertEquals(0, status.getErrorCode());
    assertEquals("", status.getErrorMessage());
  }

  @Test
  void statusNameUsedForBothNameAndState() {
    ConnectionGetQueryStatusResponse response =
        ConnectionGetQueryStatusResponse.newBuilder().setStatusName("RESUMING_WAREHOUSE").build();

    QueryStatus status = QueryStatusMapper.fromCoreResponse(response);

    assertEquals("RESUMING_WAREHOUSE", status.getName());
    assertEquals("RESUMING_WAREHOUSE", status.getState());
    assertEquals("RESUMING_WAREHOUSE", status.getDescription());
  }
}
