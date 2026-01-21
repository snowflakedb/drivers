package net.snowflake.client.internal.core.arrow;

import java.math.BigDecimal;
import net.snowflake.client.jdbc.ErrorCode;
import net.snowflake.client.jdbc.SFException;
import net.snowflake.client.jdbc.SnowflakeType;
import net.snowflake.client.jdbc.SnowflakeUtil;
import org.apache.arrow.vector.ValueVector;

public class BigIntToScaledFixedConverter extends BigIntToFixedConverter {
  public BigIntToScaledFixedConverter(
      ValueVector fieldVector, int columnIndex, DataConversionContext context, int scale) {
    super(fieldVector, columnIndex, context);
    logicalTypeStr =
        String.format(
            "%s(%s,%s)",
            SnowflakeType.FIXED,
            fieldVector.getField().getMetadata().get("precision"),
            fieldVector.getField().getMetadata().get("scale"));
    sfScale = scale;
  }

  @Override
  public float toFloat(int index) throws SFException {
    return (float) toDouble(index);
  }

  @Override
  public double toDouble(int index) throws SFException {
    if (isNull(index)) {
      return 0;
    }
    if (sfScale > 9) {
      return toBigDecimal(index).doubleValue();
    }
    double res = getLong(index);
    res = res / ArrowResultUtil.powerOfTen(sfScale);
    return res;
  }

  @Override
  public short toShort(int index) throws SFException {
    if (isNull(index)) {
      return 0;
    }
    BigDecimal val = toBigDecimal(index);
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT,
        logicalTypeStr,
        SnowflakeUtil.SHORT_STR,
        val.toPlainString());
  }

  @Override
  public int toInt(int index) throws SFException {
    if (isNull(index)) {
      return 0;
    }
    BigDecimal val = toBigDecimal(index);
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT,
        logicalTypeStr,
        SnowflakeUtil.INT_STR,
        val.toPlainString());
  }

  @Override
  public long toLong(int index) throws SFException {
    if (isNull(index)) {
      return 0;
    }
    BigDecimal val = toBigDecimal(index);
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT,
        logicalTypeStr,
        SnowflakeUtil.LONG_STR,
        val.toPlainString());
  }

  @Override
  public Object toObject(int index) {
    return toBigDecimal(index);
  }

  @Override
  public String toString(int index) {
    return isNull(index) ? null : BigDecimal.valueOf(getLong(index), sfScale).toPlainString();
  }

  @Override
  public boolean toBoolean(int index) throws SFException {
    if (isNull(index)) {
      return false;
    }
    BigDecimal val = toBigDecimal(index);
    if (val.compareTo(BigDecimal.ZERO) == 0) {
      return false;
    } else if (val.compareTo(BigDecimal.ONE) == 0) {
      return true;
    }
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT,
        logicalTypeStr,
        SnowflakeUtil.BOOLEAN_STR,
        val.toPlainString());
  }

  @Override
  public byte[] toBytes(int index) {
    if (isNull(index)) {
      return null;
    }
    byteBuf.putLong(0, getLong(index));
    return byteBuf.array();
  }
}
