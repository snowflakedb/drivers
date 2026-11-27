package net.snowflake.client.internal.api.implementation.telemetry;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyLong;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import net.snowflake.client.internal.api.implementation.exception.CoreException;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.TelemetrySendResponse;
import org.junit.jupiter.api.Test;
import org.mockito.ArgumentCaptor;

class ApplicationTelemetryTest {

  private final CoreDriverApi core = mock(CoreDriverApi.class);
  private final ConnectionHandle handle = ConnectionHandle.newBuilder().setId(7).build();
  private final ApplicationTelemetry telemetry = new ApplicationTelemetry(core, handle);

  @Test
  void shouldTagEventWithSourceAndForwardToCore() {
    when(core.telemetrySendLog(eq(handle), any(), anyLong()))
        .thenReturn(TelemetrySendResponse.getDefaultInstance());

    telemetry.submit("{\"a\":1}");

    ArgumentCaptor<String> message = ArgumentCaptor.forClass(String.class);
    verify(core).telemetrySendLog(eq(handle), message.capture(), anyLong());
    assertEquals(
        "{\"a\":1,\"source\":\"SnowflakeConnection.submitTelemetry\"}", message.getValue());
  }

  @Test
  void shouldOverwriteCallerSuppliedSource() {
    when(core.telemetrySendLog(eq(handle), any(), anyLong()))
        .thenReturn(TelemetrySendResponse.getDefaultInstance());

    telemetry.submit("{\"source\":\"spoofed\"}");

    ArgumentCaptor<String> message = ArgumentCaptor.forClass(String.class);
    verify(core).telemetrySendLog(eq(handle), message.capture(), anyLong());
    assertEquals("{\"source\":\"SnowflakeConnection.submitTelemetry\"}", message.getValue());
  }

  @Test
  void shouldDropInvalidJson() {
    telemetry.submit("{not json");

    verify(core, never()).telemetrySendLog(any(), any(), anyLong());
  }

  @Test
  void shouldDropNonObjectJson() {
    telemetry.submit("[1,2]");
    telemetry.submit("\"text\"");
    telemetry.submit("null");

    verify(core, never()).telemetrySendLog(any(), any(), anyLong());
  }

  @Test
  void shouldDropPayloadOverSizeLimit() {
    StringBuilder payload = new StringBuilder("{\"k\":\"");
    for (int i = 0; i < ApplicationTelemetry.MAX_PAYLOAD_LENGTH; i++) {
      payload.append('x');
    }
    payload.append("\"}");

    telemetry.submit(payload.toString());

    verify(core, never()).telemetrySendLog(any(), any(), anyLong());
  }

  @Test
  void shouldSwallowCoreFailure() {
    when(core.telemetrySendLog(any(), any(), anyLong())).thenThrow(mock(CoreException.class));

    telemetry.submit("{}");

    verify(core).telemetrySendLog(eq(handle), any(), anyLong());
  }
}
