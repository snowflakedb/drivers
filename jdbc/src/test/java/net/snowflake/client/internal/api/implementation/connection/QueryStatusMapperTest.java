package net.snowflake.client.internal.api.implementation.connection;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import net.snowflake.client.api.resultset.QueryStatus;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetQueryStatusResponse;
import org.junit.jupiter.api.Test;

class QueryStatusMapperTest {

  @Test
  void mapsSuccessStatusWithAllFields() {
    ConnectionGetQueryStatusResponse response =
        ConnectionGetQueryStatusResponse.newBuilder()
            .setStatusName("SUCCESS")
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
  void mapsAbortedStatus() {
    ConnectionGetQueryStatusResponse response =
        ConnectionGetQueryStatusResponse.newBuilder().setStatusName("ABORTED").build();

    QueryStatus status = QueryStatusMapper.fromCoreResponse(response);

    assertTrue(status.isAnError());
    assertFalse(status.isStillRunning());
    assertFalse(status.isSuccess());
  }

  @Test
  void defaultsOptionalFieldsWhenAbsent() {
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
  }

  @Test
  void mapsNameAndStateSeparately() {
    ConnectionGetQueryStatusResponse response =
        ConnectionGetQueryStatusResponse.newBuilder()
            .setStatusName("RESUMING_WAREHOUSE")
            .setState("compiling")
            .build();

    QueryStatus status = QueryStatusMapper.fromCoreResponse(response);

    assertEquals("RESUMING_WAREHOUSE", status.getName());
    assertEquals("compiling", status.getState());
  }
}
