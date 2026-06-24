package net.snowflake.client.internal.util;

import java.sql.Types;
import net.snowflake.client.api.resultset.SnowflakeType;

public class SnowflakeUtil {
  public static final String BIG_DECIMAL_STR = "big decimal";
  public static final String FLOAT_STR = "float";
  public static final String DOUBLE_STR = "double";
  public static final String BOOLEAN_STR = "boolean";
  public static final String SHORT_STR = "short";
  public static final String INT_STR = "int";
  public static final String LONG_STR = "long";
  public static final String TIME_STR = "time";
  public static final String TIMESTAMP_STR = "timestamp";
  public static final String DATE_STR = "date";
  public static final String BYTE_STR = "byte";
  public static final String BYTES_STR = "byte array";

  public static int toSqlType(SnowflakeType sfType) {
    if (sfType == null) {
      return Types.OTHER;
    }
    // TODO: Other types will be handled later
    switch (sfType) {
      case TEXT:
      case CHAR:
      case VARIANT:
        return Types.VARCHAR;
      case FIXED:
      case DECFLOAT:
        return Types.DECIMAL;
      case REAL:
        return Types.DOUBLE;
      case BOOLEAN:
        return Types.BOOLEAN;
      case BINARY:
        return Types.BINARY;
      case DATE:
        return Types.DATE;
      default:
        return Types.OTHER;
    }
  }
}
