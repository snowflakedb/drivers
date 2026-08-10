package net.snowflake.client.internal.util;

import java.math.BigDecimal;
import java.sql.Time;
import java.sql.Timestamp;
import java.sql.Types;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.implementation.exception.SFSQLFeatureNotSupportedException;

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

  /**
   * Convert milliseconds since epoch to whole seconds, rounding toward negative infinity for
   * pre-epoch values. Mirrors snowflake-jdbc's {@code SnowflakeUtil.getSecondsFromMillis}: negative
   * values must round to the next more-negative second (so the leftover fraction stays positive),
   * while positive values truncate. Used by the timezone-carrying timestamp/time wrappers when
   * rebuilding a {@code LocalDateTime} from epoch millis + nanos.
   */
  public static long getSecondsFromMillis(long millis) {
    long returnVal;
    if (millis < 0) {
      returnVal = (long) Math.ceil((double) Math.abs(millis) / 1000);
      returnVal *= -1;
    } else {
      returnVal = millis / 1000;
    }
    return returnVal;
  }

  // ported from snowflake-jdbc
  public static int toSqlType(SnowflakeType sfType) {
    if (sfType == null) {
      return Types.OTHER;
    }
    switch (sfType) {
      case TEXT:
      case VARIANT:
      case GEOGRAPHY:
      case GEOMETRY:
        return Types.VARCHAR;

      case CHAR:
        return Types.CHAR;

      case INTEGER:
        return Types.INTEGER;

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

      case TIME:
        return Types.TIME;

      case TIMESTAMP:
      case TIMESTAMP_NTZ:
      case TIMESTAMP_LTZ:
        return Types.TIMESTAMP;

      case TIMESTAMP_TZ:
        return Types.TIMESTAMP_WITH_TIMEZONE;

      case VECTOR:
        return SnowflakeType.EXTRA_TYPES_VECTOR;

      case ARRAY:
        return Types.ARRAY;

      case OBJECT:
      case MAP:
        return Types.STRUCT;

      default:
        return Types.OTHER;
    }
  }

  // ported from snowflake-jdbc
  public static String javaTypeToClassName(int type) {
    switch (type) {
      case Types.VARCHAR:
      case Types.CHAR:
      case Types.STRUCT:
      case Types.ARRAY:
        return String.class.getName();

      case Types.BINARY:
        return SnowflakeTypeHelper.BINARY_CLASS_NAME;

      case Types.INTEGER:
        return Integer.class.getName();

      case Types.DECIMAL:
        return BigDecimal.class.getName();

      case Types.DOUBLE:
        return Double.class.getName();

      case Types.TIMESTAMP:
      case Types.TIMESTAMP_WITH_TIMEZONE:
        return Timestamp.class.getName();

      case Types.DATE:
        return java.sql.Date.class.getName();

      case Types.TIME:
        return Time.class.getName();

      case Types.BOOLEAN:
        return Boolean.class.getName();

      case Types.BIGINT:
        return Long.class.getName();

      case Types.SMALLINT:
        return Short.class.getName();

      default:
        throw new SFSQLFeatureNotSupportedException(
            String.format("No corresponding Java type is found for java.sql.Type: %d", type));
    }
  }
}
