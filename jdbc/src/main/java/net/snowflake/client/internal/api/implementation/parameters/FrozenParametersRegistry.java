package net.snowflake.client.internal.api.implementation.parameters;

import java.io.Serializable;
import java.util.Collections;
import java.util.HashMap;
import java.util.Map;
import lombok.EqualsAndHashCode;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConfigSetting;

/** {@link ParametersRegistry} backed by an immutable snapshot of parameters. */
@EqualsAndHashCode
public final class FrozenParametersRegistry implements ParametersRegistry, Serializable {

  private static final long serialVersionUID = 1L;

  private final Map<String, ConfigSetting> parameters;

  public FrozenParametersRegistry(Map<String, ConfigSetting> parameters) {
    this.parameters =
        parameters == null
            ? Collections.emptyMap()
            : Collections.unmodifiableMap(new HashMap<>(parameters));
  }

  @Override
  public ConfigSetting getTypedValue(Property param) {
    return parameters.get(param.getKey());
  }

  @Override
  public FrozenParametersRegistry freeze() {
    return this;
  }
}
