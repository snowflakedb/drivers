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
    "decimal, " + Types.DECIMAL,
    "INT, " + Types.INTEGER,
    "integer, " + Types.INTEGER,
    "byteint, " + Types.INTEGER,
    "tinyint, " + Types.TINYINT,
    "smallint, " + Types.SMALLINT,
    "bigint, " + Types.BIGINT,
    "float, " + Types.FLOAT,
    "float4, " + Types.FLOAT,
    "FLOAT8, " + Types.FLOAT,
    "double, " + Types.DOUBLE,
    "'double precision', " + Types.DOUBLE,
    "real, " + Types.REAL,
    "char, " + Types.CHAR,
    "character, " + Types.CHAR,
    "'CHAR(10)', " + Types.CHAR,
    "VARCHAR, " + Types.VARCHAR,
    "string, " + Types.VARCHAR,
    "text, " + Types.VARCHAR,
    "BINARY, " + Types.BINARY,
    "varbinary, " + Types.VARBINARY,
    "'VARBINARY(16) NOT NULL', " + Types.VARBINARY,
    "BOOLEAN, " + Types.BOOLEAN,
    "date, " + Types.DATE,
    "time, " + Types.TIME,
    "'TIME(9)', " + Types.TIME,
    "TIMESTAMP_NTZ, " + Types.TIMESTAMP,
    "timestamp, " + Types.TIMESTAMP,
    "datetime, " + Types.TIMESTAMP,
    "timestamp_ltz, " + Types.TIMESTAMP_WITH_TIMEZONE,
    "timestamp_tz, " + Types.TIMESTAMP_WITH_TIMEZONE,
    "'TIMESTAMP_TZ(9) NOT NULL', " + Types.TIMESTAMP_WITH_TIMEZONE,
    "DECFLOAT, " + SnowflakeType.EXTRA_TYPES_DECFLOAT,
    "interval_year_month, " + SnowflakeType.EXTRA_TYPES_YEAR_MONTH_INTERVAL,
    "interval_day_time, " + SnowflakeType.EXTRA_TYPES_DAY_TIME_INTERVAL,
    "variant, " + Types.OTHER,
    "object, " + Types.JAVA_OBJECT,
    "vector, " + SnowflakeType.EXTRA_TYPES_VECTOR,
    "'VECTOR(FLOAT, 3)', " + SnowflakeType.EXTRA_TYPES_VECTOR,
    "array, " + Types.ARRAY,
    "geography, " + Types.OTHER,
    "unknown_custom_type, " + Types.OTHER,
    "'', " + Types.NULL,
    "'   ', " + Types.NULL,
    "'NUMBER NOT NULL', " + Types.NUMERIC
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
