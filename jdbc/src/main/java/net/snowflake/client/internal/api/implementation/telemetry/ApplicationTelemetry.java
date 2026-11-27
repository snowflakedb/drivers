package net.snowflake.client.internal.api.implementation.telemetry;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.internal.api.implementation.exception.CoreException;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;

/**
 * Forwards events from {@code SnowflakeConnection.submitTelemetry} to core. Events that are
 * oversized, not a JSON object, or rejected by core are dropped and logged at DEBUG by exception
 * type only, because exception messages can quote the payload.
 */
@RequiredArgsConstructor
public final class ApplicationTelemetry {

  static final int MAX_PAYLOAD_LENGTH = 64 * 1024;
  static final String SOURCE_KEY = "source";
  static final String SOURCE_VALUE = "SnowflakeConnection.submitTelemetry";

  private static final SFLogger logger = SFLoggerFactory.getLogger(ApplicationTelemetry.class);
  private static final ObjectMapper OBJECT_MAPPER = new ObjectMapper();

  private final CoreDriverApi coreDriverApi;
  private final ConnectionHandle connectionHandle;

  public void submit(String jsonPayload) {
    if (jsonPayload.length() > MAX_PAYLOAD_LENGTH) {
      logger.debug(
          "submitTelemetry: discarded payload exceeding {} code units (actual: {})",
          MAX_PAYLOAD_LENGTH,
          jsonPayload.length());
      return;
    }
    ObjectNode event = parseObject(jsonPayload);
    if (event == null) {
      return;
    }
    event.put(SOURCE_KEY, SOURCE_VALUE);
    try {
      coreDriverApi.telemetrySendLog(
          connectionHandle, event.toString(), System.currentTimeMillis());
    } catch (CoreException e) {
      logger.debug("submitTelemetry: dropped event after core call failed");
    }
  }

  private static ObjectNode parseObject(String jsonPayload) {
    JsonNode node;
    try {
      node = OBJECT_MAPPER.readTree(jsonPayload);
    } catch (JsonProcessingException e) {
      logger.debug(
          "submitTelemetry: discarded payload that is not valid JSON: {}",
          e.getClass().getSimpleName());
      return null;
    }
    if (!(node instanceof ObjectNode)) {
      logger.debug("submitTelemetry: discarded payload that is not a JSON object");
      return null;
    }
    return (ObjectNode) node;
  }
}
