package net.snowflake.client.internal.api.implementation.telemetry;

import lombok.RequiredArgsConstructor;
import net.snowflake.client.internal.api.decorator.Telemetry;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;

/**
 * Connection-backed {@link Telemetry} emitter: forwards the two wrapper signals to core via {@link
 * CoreDriverApi}, tagged with this connection's {@link ConnectionHandle}. sf_core owns spans,
 * timing, batching, and transport; the decorator hands this the raw throwable, and this layer
 * classifies it (via {@link ErrorSource#of}) so only the wire-safe strings reach core.
 *
 * <p>Fire-and-forget and best-effort: every call swallows all throwables (including the checked
 * {@link java.sql.SQLException} the core RPCs declare) so telemetry can never break the user's
 * workflow. A swallowed failure is logged at DEBUG with the throwable's class name only.
 */
@RequiredArgsConstructor
public final class CoreTelemetry implements Telemetry {

  private static final SFLogger logger = SFLoggerFactory.getLogger(CoreTelemetry.class);

  private final CoreDriverApi coreDriverApi;
  private final ConnectionHandle connectionHandle;

  @Override
  public void recordApiUsage(String apiMethod) {
    try {
      coreDriverApi.telemetrySendApiUsage(connectionHandle, apiMethod);
    } catch (Throwable t) {
      logger.debug("Swallowed telemetry api-usage failure: {}", t.getClass().getName());
    }
  }

  @Override
  public void recordWrapperError(Throwable error) {
    try {
      coreDriverApi.telemetrySendWrapperError(
          connectionHandle, error.getClass().getSimpleName(), ErrorSource.of(error).getWireValue());
    } catch (Throwable t) {
      logger.debug("Swallowed telemetry wrapper-error failure: {}", t.getClass().getName());
    }
  }
}
