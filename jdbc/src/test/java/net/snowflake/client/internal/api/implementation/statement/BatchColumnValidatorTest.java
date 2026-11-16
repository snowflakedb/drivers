package net.snowflake.client.internal.api.implementation.statement;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.Arrays;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.api.implementation.statement.PreparedStatementBindingSerializer.ParameterValue;
import org.junit.jupiter.api.Test;

class BatchColumnValidatorTest {

  @Test
  void shouldNameParameterTypesAndRowsWhenNonNullColumnChangesType() {
    ParameterValue existingIntColumn =
        new ParameterValue(SnowflakeType.INTEGER, Arrays.asList(null, "42"));
    ParameterValue newTextScalar = new ParameterValue(SnowflakeType.TEXT, "hello");

    SFSQLException thrown =
        assertThrows(
            SFSQLException.class,
            () -> BatchColumnValidator.validate(2, existingIntColumn, newTextScalar));

    assertEquals(ErrorCode.ARRAY_BIND_MIXED_TYPES_NOT_SUPPORTED, thrown.getErrorCode());
    assertEquals(
        ErrorCode.ARRAY_BIND_MIXED_TYPES_NOT_SUPPORTED.getSqlState(),
        thrown.toSQLException().getSQLState());
    assertEquals(
        "Array binding does not support mixed types: parameter 2 was bound with type INTEGER"
            + " in row 2 and type TEXT in row 3",
        thrown.getMessage());
  }
}
