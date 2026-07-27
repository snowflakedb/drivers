package net.snowflake.client.internal.api.implementation.parameters;

import java.io.Serializable;
import java.util.Collections;
import java.util.HashMap;
import java.util.Map;
import lombok.EqualsAndHashCode;

/** {@link ParametersRegistry} backed by an immutable snapshot of parameters. */
@EqualsAndHashCode
public final class FrozenParametersRegistry implements ParametersRegistry, Serializable {

  private static final long serialVersionUID = 1L;

  private final Map<String, String> parameters;

  public FrozenParametersRegistry(Map<String, String> parameters) {
    this.parameters =
        parameters == null
            ? Collections.emptyMap()
            : Collections.unmodifiableMap(new HashMap<>(parameters));
  }

  @Override
  public String getRawValue(Property param, String defaultValue) {
    return parameters.get(param.getKey());
  }

  @Override
  public FrozenParametersRegistry freeze() {
    return this;
  }
}
