package net.snowflake.client.internal.unicore;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.util.Collections;
import java.util.Map;
import java.util.concurrent.Future;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverService;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConfigSetting;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetOptionsRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionSetOptionsResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ExecuteQueryResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementExecuteQueryRequest;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.StatementHandle;
import org.junit.jupiter.api.Test;
import org.mockito.ArgumentCaptor;

class CoreDriverApiImplTest {

  @Test
  void shouldForwardWhetherCoreMustResolveTheDefaultConnectionProfile() throws Exception {
    DatabaseDriverService client = mock(DatabaseDriverService.class);
    when(client.connectionSetOptions(any()))
        .thenReturn(ConnectionSetOptionsResponse.getDefaultInstance());
    CoreDriverApiImpl api = new CoreDriverApiImpl(client);
    ConnectionHandle handle = ConnectionHandle.newBuilder().setId(42).setMagic(99).build();
    Map<String, ConfigSetting> options =
        Collections.singletonMap(
            "warehouse", ConfigSetting.newBuilder().setStringValue("TEST_WH").build());
    ArgumentCaptor<ConnectionSetOptionsRequest> requests =
        ArgumentCaptor.forClass(ConnectionSetOptionsRequest.class);

    api.connectionSetOptions(handle, options);
    api.connectionSetOptionsForDefaultProfile(handle, options);

    verify(client, times(2)).connectionSetOptions(requests.capture());
    assertFalse(requests.getAllValues().get(0).getNoConnectionDetails());
    assertTrue(requests.getAllValues().get(1).getNoConnectionDetails());
    for (ConnectionSetOptionsRequest request : requests.getAllValues()) {
      assertEquals(handle, request.getConnHandle());
      assertEquals(options, request.getOptionsMap());
    }
  }

  @Test
  void shouldReturnTheInFlightQueryFutureWithoutBlocking() throws Exception {
    DatabaseDriverService client = mock(DatabaseDriverService.class);
    @SuppressWarnings("unchecked")
    Future<ExecuteQueryResponse> inFlight = mock(Future.class);
    when(client.statementExecuteQuery(any())).thenReturn(inFlight);
    CoreDriverApiImpl api = new CoreDriverApiImpl(client);
    StatementHandle handle = StatementHandle.newBuilder().setId(1).setMagic(2).build();
    ArgumentCaptor<StatementExecuteQueryRequest> requests =
        ArgumentCaptor.forClass(StatementExecuteQueryRequest.class);

    Future<ExecuteQueryResponse> returned = api.statementExecuteQuery(handle, null);

    assertSame(inFlight, returned);
    verify(client).statementExecuteQuery(requests.capture());
    assertEquals(handle, requests.getValue().getStmtHandle());
    verify(inFlight, never()).get();
  }
}
