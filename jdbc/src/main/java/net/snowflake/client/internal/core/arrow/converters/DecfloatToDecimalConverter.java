package net.snowflake.client.internal.core.arrow.converters;

import java.math.BigDecimal;
import java.math.BigInteger;
import java.util.Map;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.util.SnowflakeUtil;

class DecfloatToDecimalConverter extends AbstractArrowVectorConverter {

  private static final int MAX_PLAIN_UNSIGNED_CHARACTERS = 38;

  private final Column vector;

  public DecfloatToDecimalConverter(Column vector, int idx, DataConversionContext context) {
    super(SnowflakeType.DECFLOAT.name(), vector, idx, context);
    this.vector = vector;
  }

  @Override
  public BigDecimal toBigDecimal(int index) {
    if (isNull(index)) {
      return null;
    }
    Map<String, Object> value = (Map<String, Object>) vector.getObject(index);
    byte[] significandBytes = (byte[]) value.get("significand");
    short exponent = (short) value.get("exponent");
    BigInteger significand = new BigInteger(significandBytes);
    return new BigDecimal(significand, -exponent);
  }

  @Override
  public double toDouble(int rowIndex) {
    if (isNull(rowIndex)) {
      return 0;
    }
    return toBigDecimal(rowIndex).doubleValue();
  }

  @Override
  public float toFloat(int rowIndex) {
    if (isNull(rowIndex)) {
      return 0;
    }
    return toBigDecimal(rowIndex).floatValue();
  }

  @Override
  public short toShort(int rowIndex) {
    if (isNull(rowIndex)) {
      return 0;
    }
    BigDecimal bigDecimal = toBigDecimal(rowIndex);
    if (bigDecimal.scale() == 0) {
      short shortVal = bigDecimal.shortValue();
      if (shortVal == bigDecimal.longValue()) {
        return shortVal;
      } else {
        throw SFSQLException.fromErrorCode(
            ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, "Short", bigDecimal.toPlainString());
      }
    } else {
      throw SFSQLException.fromErrorCode(
          ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, "Short", bigDecimal.toPlainString());
    }
  }

  @Override
  public int toInt(int rowIndex) {
    if (isNull(rowIndex)) {
      return 0;
    }
    BigDecimal bigDecimal = toBigDecimal(rowIndex);
    if (bigDecimal.scale() == 0) {
      int intVal = bigDecimal.intValue();
      if (intVal == bigDecimal.longValue()) {
        return intVal;
      } else {
        throw SFSQLException.fromErrorCode(
            ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, "Integer", bigDecimal.toPlainString());
      }
    } else {
      throw SFSQLException.fromErrorCode(
          ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, "Integer", bigDecimal.toPlainString());
    }
  }

  @Override
  public long toLong(int rowIndex) {
    if (isNull(rowIndex)) {
      return 0;
    }
    BigDecimal bigDecimal = toBigDecimal(rowIndex);
    if (bigDecimal.scale() == 0) {
      BigInteger intVal = bigDecimal.toBigIntegerExact();
      if (intVal.bitLength() <= 63) {
        return intVal.longValue();
      } else {
        throw SFSQLException.fromErrorCode(
            ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, "Long", bigDecimal.toPlainString());
      }
    } else {
      throw SFSQLException.fromErrorCode(
          ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, "Long", bigDecimal.toPlainString());
    }
  }

  @Override
  public Object toObject(int index) {
    return toBigDecimal(index);
  }

  @Override
  public String toString(int index) {
    if (isNull(index)) {
      return null;
    }
    return formatDecfloat(toBigDecimal(index));
  }

  private static String formatDecfloat(BigDecimal value) {
    if (value.signum() == 0) {
      return "0";
    }
    BigDecimal stripped = value.stripTrailingZeros();
    if (fitsInPlainDecimal(stripped)) {
      return stripped.toPlainString();
    }
    return toNormalizedScientific(stripped);
  }

  private static boolean fitsInPlainDecimal(BigDecimal value) {
    int precision = value.precision();
    int scale = value.scale();
    int unsignedLength;
    if (scale <= 0) {
      unsignedLength = precision - scale;
    } else if (precision > scale) {
      unsignedLength = precision + 1;
    } else {
      unsignedLength = scale + 2;
    }
    return unsignedLength <= MAX_PLAIN_UNSIGNED_CHARACTERS;
  }

  private static String toNormalizedScientific(BigDecimal value) {
    String digits = value.unscaledValue().abs().toString();
    int exponent = digits.length() - 1 - value.scale();
    StringBuilder result = new StringBuilder();
    if (value.signum() < 0) {
      result.append('-');
    }
    result.append(digits.charAt(0));
    if (digits.length() > 1) {
      result.append('.').append(digits, 1, digits.length());
    }
    result.append('e').append(exponent);
    return result.toString();
  }

  @Override
  public byte[] toBytes(int index) {
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.BYTES_STR, null);
  }

  @Override
  public boolean toBoolean(int rowIndex) {
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.BOOLEAN_STR, null);
  }
}
