package net.snowflake.client.internal.api.implementation.parameters;

import lombok.RequiredArgsConstructor;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetAllParametersResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetParameterResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;

/**
 * {@link ParametersRegistry} backed by a live connection: values are fetched from core on demand
 * via {@code connectionGetParameter}, so they always reflect the current session state (e.g. after
 * {@code ALTER SESSION}).
 */
@RequiredArgsConstructor
public class CoreParametersRegistry implements ParametersRegistry {

  private final CoreDriverApi coreDriverApi;
  private final ConnectionHandle handle;

  @Override
  public String getRawValue(Property param, String defaultValue) {
    try {
      ConnectionGetParameterResponse response =
          coreDriverApi.connectionGetParameter(handle, param.getKey());
      if (response != null && response.hasValue()) {
        return response.getValue();
      }
    } catch (RuntimeException e) {
      logger.warn(
          "Failed to read {} session parameter; defaulting to {}", param.getKey(), defaultValue, e);
    }
    return null;
  }

  @Override
  public FrozenParametersRegistry freeze() {
    try {
      ConnectionGetAllParametersResponse response =
          coreDriverApi.connectionGetAllParameters(handle);
      return new FrozenParametersRegistry(response.getParametersMap());
    } catch (RuntimeException e) {
      logger.warn("Failed to snapshot session parameters; using an empty snapshot", e);
      return ParametersRegistry.EMPTY;
    }
  }
}
