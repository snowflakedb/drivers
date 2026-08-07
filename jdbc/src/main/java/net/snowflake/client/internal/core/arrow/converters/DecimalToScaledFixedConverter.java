package net.snowflake.client.internal.core.arrow.converters;

import static net.snowflake.client.internal.core.arrow.ArrowResultUtil.getDurationFromNanos;

import java.math.BigDecimal;
import java.time.Duration;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.util.SnowflakeUtil;
import org.apache.arrow.vector.DecimalVector;
import org.apache.arrow.vector.ValueVector;

/**
 * Data vector whose snowflake logical type is fixed while represented as a BigDecimal value vector.
 */
public class DecimalToScaledFixedConverter extends AbstractArrowVectorConverter {
  protected final DecimalVector decimalVector;

  public DecimalToScaledFixedConverter(
      ValueVector fieldVector, int vectorIndex, DataConversionContext context) {
    super(
        String.format(
            "%s(%s,%s)",
            SnowflakeType.FIXED,
            fieldVector.getField().getMetadata().get("precision"),
            fieldVector.getField().getMetadata().get("scale")),
        fieldVector,
        vectorIndex,
        context);
    decimalVector = (DecimalVector) fieldVector;
  }

  @Override
  public byte[] toBytes(int index) {
    if (isNull(index)) {
      return null;
    }
    return toBigDecimal(index).toBigInteger().toByteArray();
  }

  @Override
  public byte toByte(int index) {
    if (isNull(index)) {
      return 0;
    }
    BigDecimal bigDecimal = toBigDecimal(index);
    if (bigDecimal.scale() == 0) {
      byte byteVal = bigDecimal.byteValue();
      if (byteVal == bigDecimal.longValue()) {
        return byteVal;
      }
      throw SFSQLException.fromErrorCode(
          ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, "Byte", bigDecimal.toPlainString());
    }
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, "Byte", bigDecimal.toPlainString());
  }

  @Override
  public short toShort(int index) {
    if (isNull(index)) {
      return 0;
    }
    BigDecimal bigDecimal = toBigDecimal(index);
    if (bigDecimal.scale() == 0) {
      short shortValue = bigDecimal.shortValue();
      if (bigDecimal.compareTo(BigDecimal.valueOf(shortValue)) == 0) {
        return shortValue;
      }
      throw SFSQLException.fromErrorCode(
          ErrorCode.INVALID_VALUE_CONVERT,
          logicalTypeStr,
          SnowflakeUtil.SHORT_STR,
          bigDecimal.toPlainString());
    }
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT,
        logicalTypeStr,
        SnowflakeUtil.SHORT_STR,
        bigDecimal.toPlainString());
  }

  @Override
  public int toInt(int index) {
    if (isNull(index)) {
      return 0;
    }
    BigDecimal bigDecimal = toBigDecimal(index);
    if (bigDecimal.scale() == 0) {
      int intValue = bigDecimal.intValue();
      if (bigDecimal.compareTo(BigDecimal.valueOf(intValue)) == 0) {
        return intValue;
      }
      throw SFSQLException.fromErrorCode(
          ErrorCode.INVALID_VALUE_CONVERT,
          logicalTypeStr,
          SnowflakeUtil.INT_STR,
          bigDecimal.toPlainString());
    }
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT,
        logicalTypeStr,
        SnowflakeUtil.INT_STR,
        bigDecimal.toPlainString());
  }

  @Override
  public long toLong(int index) {
    if (isNull(index)) {
      return 0;
    }
    BigDecimal bigDecimal = toBigDecimal(index);
    if (bigDecimal.scale() == 0) {
      long longValue = bigDecimal.longValue();
      if (bigDecimal.compareTo(BigDecimal.valueOf(longValue)) == 0) {
        return longValue;
      }
      throw SFSQLException.fromErrorCode(
          ErrorCode.INVALID_VALUE_CONVERT,
          logicalTypeStr,
          SnowflakeUtil.LONG_STR,
          bigDecimal.toPlainString());
    }
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT,
        logicalTypeStr,
        SnowflakeUtil.LONG_STR,
        bigDecimal.toPlainString());
  }

  @Override
  public float toFloat(int index) {
    if (isNull(index)) {
      return 0;
    }
    return toBigDecimal(index).floatValue();
  }

  @Override
  public double toDouble(int index) {
    if (isNull(index)) {
      return 0;
    }
    return toBigDecimal(index).doubleValue();
  }

  @Override
  public BigDecimal toBigDecimal(int index) {
    return decimalVector.getObject(index);
  }

  @Override
  public Object toObject(int index) {
    return toBigDecimal(index);
  }

  @Override
  public String toString(int index) {
    BigDecimal bigDecimal = toBigDecimal(index);
    return bigDecimal == null ? null : bigDecimal.toPlainString();
  }

  @Override
  public boolean toBoolean(int index) {
    if (isNull(index)) {
      return false;
    }
    BigDecimal val = toBigDecimal(index);
    if (val.compareTo(BigDecimal.ZERO) == 0) {
      return false;
    } else if (val.compareTo(BigDecimal.ONE) == 0) {
      return true;
    }
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT,
        logicalTypeStr,
        SnowflakeUtil.BOOLEAN_STR,
        val.toPlainString());
  }

  @Override
  public Duration toDuration(int index) {
    if (isNull(index)) {
      return null;
    }
    BigDecimal numNanos = toBigDecimal(index);
    try {
      return getDurationFromNanos(numNanos);
    } catch (ArithmeticException e) {
      throw SFSQLException.fromErrorCode(
          ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, "Duration", numNanos.toPlainString());
    }
  }
}
