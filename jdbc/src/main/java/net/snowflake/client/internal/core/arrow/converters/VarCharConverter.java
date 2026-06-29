package net.snowflake.client.internal.core.arrow.converters;

import java.math.BigDecimal;
import java.nio.charset.StandardCharsets;
import java.sql.Date;
import java.text.DateFormat;
import java.text.ParseException;
import java.text.SimpleDateFormat;
import java.util.TimeZone;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SFException;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.util.SnowflakeUtil;
import org.apache.arrow.vector.ValueVector;
import org.apache.arrow.vector.VarCharVector;

public class VarCharConverter extends AbstractArrowVectorConverter {
  // Parsed once and cloned per call: SimpleDateFormat is not thread-safe and re-parsing the pattern
  // on every row (VARCHAR scans can be large) is needless GC churn. The pattern, and its lenient
  // parsing, mirror snowflake-jdbc's VarCharConverter exactly.
  private static final SimpleDateFormat DATE_FORMAT_TEMPLATE = new SimpleDateFormat("yyyy-MM-dd");

  private final VarCharVector varCharVector;

  public VarCharConverter(ValueVector valueVector, int columnIndex, DataConversionContext context) {
    super(SnowflakeType.TEXT.name(), valueVector, columnIndex, context);
    this.varCharVector = (VarCharVector) valueVector;
  }

  @Override
  public String toString(int index) {
    byte[] bytes = toBytes(index);
    return bytes == null ? null : new String(bytes, StandardCharsets.UTF_8);
  }

  @Override
  public byte[] toBytes(int index) {
    return isNull(index) ? null : varCharVector.get(index);
  }

  @Override
  public Object toObject(int index) {
    return toString(index);
  }

  @Override
  public short toShort(int index) throws SFException {
    String str = toString(index);
    try {
      if (str == null) {
        return 0;
      }
      return Short.parseShort(str);
    } catch (NumberFormatException ex) {
      throw new SFException(
          ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.SHORT_STR, str);
    }
  }

  @Override
  public int toInt(int index) throws SFException {
    String str = toString(index);
    try {
      if (str == null) {
        return 0;
      }
      return Integer.parseInt(str);
    } catch (NumberFormatException ex) {
      throw new SFException(
          ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.INT_STR, str);
    }
  }

  @Override
  public long toLong(int index) throws SFException {
    String str = toString(index);
    try {
      if (str == null) {
        return 0;
      }
      return Long.parseLong(str);
    } catch (NumberFormatException ex) {
      throw new SFException(
          ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.LONG_STR, str);
    }
  }

  @Override
  public float toFloat(int index) throws SFException {
    String str = toString(index);
    try {
      if (str == null) {
        return 0;
      }
      return Float.parseFloat(str);
    } catch (NumberFormatException ex) {
      throw new SFException(
          ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.FLOAT_STR, str);
    }
  }

  @Override
  public double toDouble(int index) throws SFException {
    String str = toString(index);
    try {
      if (str == null) {
        return 0;
      }
      return Double.parseDouble(str);
    } catch (NumberFormatException ex) {
      throw new SFException(
          ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.DOUBLE_STR, str);
    }
  }

  @Override
  public BigDecimal toBigDecimal(int index) throws SFException {
    String str = toString(index);
    try {
      if (str == null) {
        return null;
      }
      return new BigDecimal(str);
    } catch (Exception ex) {
      throw new SFException(
          ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.BIG_DECIMAL_STR, str);
    }
  }

  @Override
  public boolean toBoolean(int index) throws SFException {
    String str = toString(index);
    if (str == null) {
      return false;
    } else if ("0".equals(str) || Boolean.FALSE.toString().equalsIgnoreCase(str)) {
      return false;
    } else if ("1".equals(str) || Boolean.TRUE.toString().equalsIgnoreCase(str)) {
      return true;
    } else {
      throw new SFException(
          ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.BOOLEAN_STR, str);
    }
  }

  @Override
  public Date toDate(int index, TimeZone jvmTz, boolean useDateFormat) throws SFException {
    if (isNull(index)) {
      return null;
    }
    // NOTE: jvmTz and useDateFormat are intentionally ignored, matching snowflake-jdbc's
    // VarCharConverter. The string is anchored at midnight in the JVM default timezone, so a
    // caller-supplied Calendar zone has no effect on a VARCHAR-typed DATE column (unlike
    // DateConverter, which is zone-aware). Parsing is also lenient (legacy parity): inputs like
    // "2024-13-45" roll over and a valid prefix of "2024-01-15garbage" is accepted rather than
    // rejected with INVALID_VALUE_CONVERT.
    try {
      DateFormat dateFormat = (DateFormat) DATE_FORMAT_TEMPLATE.clone();
      return new Date(dateFormat.parse(toString(index)).getTime());
    } catch (ParseException e) {
      throw new SFException(
          ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.DATE_STR, "");
    }
  }
}
