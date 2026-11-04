package net.snowflake.client.internal.api.implementation.connection;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import net.snowflake.client.api.resultset.QueryStatus;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetQueryStatusResponse;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.CsvSource;

class QueryStatusMapperTest {

  @Test
  void shouldMapSuccessStatusWithAllFields() {
    ConnectionGetQueryStatusResponse response =
        ConnectionGetQueryStatusResponse.newBuilder()
            .setStatusName("SUCCESS")
            .setState("running")
            .setEndTime(1700000000L)
            .setStartTime(1699999990L)
            .setTotalDuration(10000)
            .setQueryId("01abc-query-id")
            .setSessionId(42L)
            .setSqlText("SELECT 1")
            .setWarehouseId(100)
            .setWarehouseName("COMPUTE_WH")
            .setWarehouseExternalSize("X-Small")
            .setWarehouseServerType("STANDARD")
            .build();

    QueryStatus status = QueryStatusMapper.fromCoreResponse(response);

    assertEquals("SUCCESS", status.getName());
    assertEquals("running", status.getState());
    assertTrue(status.isSuccess());
    assertFalse(status.isStillRunning());
    assertEquals(1700000000L, status.getEndTime());
    assertEquals(1699999990L, status.getStartTime());
    assertEquals(10000, status.getTotalDuration());
    assertEquals("01abc-query-id", status.getId());
    assertEquals(42L, status.getSessionId());
    assertEquals("SELECT 1", status.getSqlText());
    assertEquals(100, status.getWarehouseId());
    assertEquals("COMPUTE_WH", status.getWarehouseName());
    assertEquals("X-Small", status.getWarehouseExternalSize());
    assertEquals("STANDARD", status.getWarehouseServerType());
  }

  @Test
  void shouldMapRunningStatus() {
    ConnectionGetQueryStatusResponse response =
        ConnectionGetQueryStatusResponse.newBuilder().setStatusName("RUNNING").build();

    QueryStatus status = QueryStatusMapper.fromCoreResponse(response);

    assertEquals("RUNNING", status.getName());
    assertFalse(status.isSuccess());
    assertTrue(status.isStillRunning());
    assertFalse(status.isAnError());
  }

  @Test
  void shouldMapFailedWithErrorAndErrorFields() {
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
  void shouldMapQueuedStatus() {
    ConnectionGetQueryStatusResponse response =
        ConnectionGetQueryStatusResponse.newBuilder().setStatusName("QUEUED").build();

    QueryStatus status = QueryStatusMapper.fromCoreResponse(response);

    assertTrue(status.isStillRunning());
    assertFalse(status.isSuccess());
  }

  @Test
  void shouldMapAbortedStatus() {
    ConnectionGetQueryStatusResponse response =
        ConnectionGetQueryStatusResponse.newBuilder().setStatusName("ABORTED").build();

    QueryStatus status = QueryStatusMapper.fromCoreResponse(response);

    assertTrue(status.isAnError());
    assertFalse(status.isStillRunning());
    assertFalse(status.isSuccess());
  }

  @Test
  void shouldDefaultErrorFieldsWhenAbsent() {
    ConnectionGetQueryStatusResponse response =
        ConnectionGetQueryStatusResponse.newBuilder().setStatusName("RUNNING").build();

    QueryStatus status = QueryStatusMapper.fromCoreResponse(response);

    assertEquals(0, status.getErrorCode());
    assertEquals("", status.getErrorMessage());
    assertEquals(0L, status.getEndTime());
    assertEquals(0L, status.getStartTime());
    assertEquals(0, status.getTotalDuration());
    assertEquals("", status.getId());
    assertEquals(0L, status.getSessionId());
    assertEquals("", status.getSqlText());
    assertEquals(0, status.getWarehouseId());
    assertEquals("", status.getWarehouseName());
    assertEquals("", status.getWarehouseExternalSize());
    assertEquals("", status.getWarehouseServerType());
    assertEquals("", status.getState());
  }

  @ParameterizedTest
  @CsvSource({
    "QUEUED_REPARING_WAREHOUSE, QUEUED_REPAIRING_WAREHOUSE, true, false",
    "RESUMING_WAREHOUSE, RESUMING_WAREHOUSE, true, false",
    "BLOCKED, BLOCKED, true, true",
    "ABORTING, ABORTING, false, true",
    "FAILED_WITH_INCIDENT, FAILED_WITH_INCIDENT, false, true",
    "DISCONNECTED, DISCONNECTED, false, true",
    "RESTARTED, RESTARTED, false, false"
  })
  void shouldClassifyRunningAndErrorStateOfWireStatus(
      String wireName, QueryStatus.Status expected, boolean stillRunning, boolean anError) {
    QueryStatus status =
        QueryStatusMapper.fromCoreResponse(
            ConnectionGetQueryStatusResponse.newBuilder().setStatusName(wireName).build());

    assertEquals(expected, status.getStatus());
    assertEquals(stillRunning, status.isStillRunning());
    assertEquals(anError, status.isAnError());
    assertFalse(status.isSuccess());
  }

  @Test
  void shouldKeepLegacyMisspelledDescriptionForRepairingWarehouse() {
    assertEquals(9, QueryStatus.Status.QUEUED_REPAIRING_WAREHOUSE.getValue());
    assertEquals(
        "QUEUED_REPARING_WAREHOUSE",
        QueryStatus.Status.QUEUED_REPAIRING_WAREHOUSE.getDescription());
  }

  @Test
  void shouldMapUnknownWireStatusToNoDataThatIsStillRunning() {
    QueryStatus status =
        QueryStatusMapper.fromCoreResponse(
            ConnectionGetQueryStatusResponse.newBuilder().setStatusName("UNKNOWN_FUTURE").build());

    assertFalse(status.isEmpty());
    assertEquals(QueryStatus.Status.NO_DATA, status.getStatus());
    assertTrue(status.isStillRunning());
  }

  @Test
  void shouldReportEmptyStatusAsNoDataThatIsStillRunning() {
    QueryStatus empty = QueryStatus.empty();

    assertTrue(empty.isEmpty());
    assertEquals(QueryStatus.Status.NO_DATA, empty.getStatus());
    assertTrue(empty.isStillRunning());
    assertFalse(empty.isSuccess());
    assertFalse(empty.isAnError());
  }

  @Test
  void shouldReturnNullStatusWhenConstructedWithNullStatusName() {
    QueryStatus status = new QueryStatus(0, 0, "", "", null, 0, "", 0, "", 0, "", 0, "", "");

    assertNull(status.getStatus());
  }
}
