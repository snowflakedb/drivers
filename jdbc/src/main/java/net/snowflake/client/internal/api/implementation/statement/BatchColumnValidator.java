package net.snowflake.client.internal.api.implementation.statement;

import java.sql.SQLException;
import java.util.List;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.implementation.statement.PreparedStatementBindingSerializer.ParameterValue;

final class BatchColumnValidator {

  private BatchColumnValidator() {}

  static void validate(int parameterIndex, ParameterValue existing, ParameterValue newValue)
      throws SQLException {
    if (newValue == null) {
      throw new SnowflakeSQLException("Missing value for parameter index: " + parameterIndex);
    }
    if (existing == null) {
      return;
    }
    String stringValue = (String) newValue.value();
    if (stringValue == null) {
      return;
    }
    SnowflakeType prevType = existing.bindType();
    SnowflakeType newType = newValue.bindType();
    if (prevType == SnowflakeType.ANY || prevType == newType) {
      return;
    }
    @SuppressWarnings("unchecked")
    List<String> values = (List<String>) existing.value();
    if (allNullsSoFar(values)) {
      return;
    }
    int prevRow = values.size();
    throw new SnowflakeSQLException(
        ErrorCode.ARRAY_BIND_MIXED_TYPES_NOT_SUPPORTED,
        "Array binding does not support mixed types: parameter "
            + parameterIndex
            + " was bound with type "
            + prevType
            + " in row "
            + prevRow
            + " and type "
            + newType
            + " in row "
            + (prevRow + 1));
  }

  private static boolean allNullsSoFar(List<String> values) {
    for (String v : values) {
      if (v != null) {
        return false;
      }
    }
    return true;
  }
}
