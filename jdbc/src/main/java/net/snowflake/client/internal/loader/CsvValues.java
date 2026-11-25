package net.snowflake.client.internal.loader;

import java.math.BigDecimal;
import java.text.DateFormat;
import java.time.Duration;
import java.time.Period;
import java.util.Date;
import net.snowflake.client.internal.util.HexUtil;

/** Serializes Loader API row values into the CSV cells that the COPY command reads. */
final class CsvValues {

  private CsvValues() {}

  /**
   * Returns a lexical value of an object that is suitable for Snowflake import serialization.
   *
   * @param o Java object representing value in Snowflake.
   * @param dateFormat java.util.Date and java.sql.Date format
   * @param timeFormat java.sql.Time format
   * @param timestampFormat first part of java.sql.Timestamp format
   * @param timestampTzFormat last part of java.sql.Timestamp format
   * @return String representation of it that can be used for creating a load file
   */
  static String lexicalValue(
      Object o,
      DateFormat dateFormat,
      DateFormat timeFormat,
      DateFormat timestampFormat,
      DateFormat timestampTzFormat) {
    if (o == null) {
      return null;
    }

    Class<?> c = o.getClass();

    if (c == Date.class || c == java.sql.Date.class) {
      return format(o, dateFormat);
    }

    if (c == java.sql.Time.class) {
      return format(o, timeFormat);
    }

    if (c == java.sql.Timestamp.class) {
      String stdFmt = o.toString();
      String nanos = stdFmt.substring(stdFmt.indexOf('.') + 1);
      String ret1 = format(o, timestampFormat);
      String ret2 = format(o, timestampTzFormat);
      return ret1 + nanos + ret2;
    }

    if (c == Double.class) {
      return Double.toHexString((Double) o);
    }

    if (c == Float.class) {
      return Float.toHexString((Float) o);
    }

    if (c == Integer.class || c == Period.class || c == Duration.class || c == BigDecimal.class) {
      return o.toString();
    }

    if (c == byte[].class) {
      return HexUtil.bytesToHex((byte[]) o);
    }

    return String.valueOf(o);
  }

  /**
   * Escapes a string value for CSV format.
   *
   * <p>A cell is quoted only when it contains a double quote, a line feed, a comma, or a backslash.
   * The COPY command reads the file with {@code FIELD_OPTIONALLY_ENCLOSED_BY='"'} and the default
   * {@code ESCAPE_UNENCLOSED_FIELD} of backslash, so an unquoted backslash would start an escape
   * sequence.
   *
   * @param value the value to escape
   * @return the escaped value: an empty field for {@code null}, {@code ""} for an empty string
   */
  static String escapeForCsv(String value) {
    if (value == null) {
      return "";
    }
    if (value.isEmpty()) {
      return "\"\"";
    }
    if (value.indexOf('"') >= 0
        || value.indexOf('\n') >= 0
        || value.indexOf(',') >= 0
        || value.indexOf('\\') >= 0) {
      return '"' + value.replaceAll("\"", "\"\"") + '"';
    } else {
      return value;
    }
  }

  private static String format(Object o, DateFormat sdf) {
    synchronized (sdf) {
      return sdf.format(o);
    }
  }
}
