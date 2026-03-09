package net.snowflake.client.internal.api.implementation.statement;

import java.util.ArrayList;
import java.util.List;

final class PreparedStatementBinding {
  static final String ANY_BIND_TYPE = "ANY";

  private final String bindType;
  private final BindingPayload payload;

  private PreparedStatementBinding(String bindType, BindingPayload payload) {
    this.bindType = bindType;
    this.payload = payload;
  }

  static PreparedStatementBinding scalar(String bindType, String value) {
    return new PreparedStatementBinding(bindType, new ScalarBindingPayload(value));
  }

  static PreparedStatementBinding arrayColumn(String bindType, List<String> values) {
    return new PreparedStatementBinding(bindType, new ArrayBindingPayload(values));
  }

  String bindType() {
    return bindType;
  }

  boolean isNull() {
    return payload.isNull();
  }

  String scalarValue() {
    if (!(payload instanceof ScalarBindingPayload)) {
      throw new IllegalStateException("Expected scalar binding payload");
    }
    return ((ScalarBindingPayload) payload).value();
  }

  Object jsonValue() {
    return payload.jsonValue();
  }

  PreparedStatementBinding copy() {
    return new PreparedStatementBinding(bindType, payload.copy());
  }

  private interface BindingPayload {
    Object jsonValue();

    boolean isNull();

    BindingPayload copy();
  }

  private static final class ScalarBindingPayload implements BindingPayload {
    private final String value;

    private ScalarBindingPayload(String value) {
      this.value = value;
    }

    private String value() {
      return value;
    }

    @Override
    public Object jsonValue() {
      return value;
    }

    @Override
    public boolean isNull() {
      return value == null;
    }

    @Override
    public BindingPayload copy() {
      return new ScalarBindingPayload(value);
    }
  }

  private static final class ArrayBindingPayload implements BindingPayload {
    private final List<String> values;

    private ArrayBindingPayload(List<String> values) {
      this.values = new ArrayList<String>(values);
    }

    @Override
    public Object jsonValue() {
      return new ArrayList<String>(values);
    }

    @Override
    public boolean isNull() {
      return false;
    }

    @Override
    public BindingPayload copy() {
      return new ArrayBindingPayload(values);
    }
  }
}
