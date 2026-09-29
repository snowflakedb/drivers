package net.snowflake.client.internal.util;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;

import java.sql.Types;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.util.SnowflakeTypeHelper.JavaSQLType;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.CsvSource;

class SnowflakeTypeHelperTest {

  @ParameterizedTest
  @CsvSource({
    "'NUMBER(38,0)', " + Types.NUMERIC,
    "numeric, " + Types.NUMERIC,
    "INT, " + Types.INTEGER,
    "integer, " + Types.INTEGER,
    "VARCHAR, " + Types.VARCHAR,
    "string, " + Types.VARCHAR,
    "TIMESTAMP_NTZ, " + Types.TIMESTAMP,
    "timestamp, " + Types.TIMESTAMP,
    "BOOLEAN, " + Types.BOOLEAN,
    "BINARY, " + Types.BINARY,
    "DECFLOAT, " + SnowflakeType.EXTRA_TYPES_DECFLOAT,
    "'', " + Types.NULL,
    "'   ', " + Types.NULL,
    "'NUMBER NOT NULL', " + Types.NUMERIC,
    "FLOAT8, " + Types.FLOAT
  })
  void shouldConvertStringToType(String typeName, int expected) {
    assertEquals(expected, SnowflakeTypeHelper.convertStringToType(typeName));
  }

  @Test
  void shouldReturnNullTypeForNullTypeName() {
    assertEquals(Types.NULL, SnowflakeTypeHelper.convertStringToType(null));
  }

  @ParameterizedTest
  @CsvSource({
    "" + Types.INTEGER + ", true",
    "" + Types.DECIMAL + ", true",
    "" + Types.DOUBLE + ", true",
    "" + Types.VARCHAR + ", false",
    "" + Types.BOOLEAN + ", false"
  })
  void shouldReportJavaTypeSigned(int type, boolean expected) {
    assertEquals(expected, SnowflakeTypeHelper.isJavaTypeSigned(type));
  }

  @ParameterizedTest
  @CsvSource({
    "" + Types.INTEGER + ", INTEGER",
    "" + Types.TIMESTAMP + ", TIMESTAMP",
    "" + Types.OTHER + ", OTHER"
  })
  void shouldFindJavaSqlType(int type, String expectedName) {
    JavaSQLType found = JavaSQLType.find(type);
    assertEquals(expectedName, found.name());
    assertEquals(type, found.getType());
  }

  @Test
  void shouldReturnNullWhenJavaSqlTypeUnknown() {
    assertNull(JavaSQLType.find(-999));
  }
}
