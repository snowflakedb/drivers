package net.snowflake.client.internal.core.arrow.converters;

import java.math.BigDecimal;
import java.nio.ByteBuffer;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.util.SnowflakeUtil;

public class TinyIntToFixedConverter extends AbstractArrowVectorConverter {
  protected final Column tinyColumn;
  protected int sfScale;
  protected ByteBuffer byteBuf = ByteBuffer.allocate(Byte.BYTES);

  public TinyIntToFixedConverter(
      Column fieldVector, int columnIndex, DataConversionContext context) {
    super(
        String.format(
            "%s(%s,%s)",
            SnowflakeType.FIXED,
            fieldVector.getField().getMetadata().get("precision"),
            fieldVector.getField().getMetadata().get("scale")),
        fieldVector,
        columnIndex,
        context);
    this.tinyColumn = fieldVector;
  }

  @Override
  public byte[] toBytes(int index) {
    if (isNull(index)) {
      return null;
    }
    byteBuf.put(0, getByte(index));
    return byteBuf.array();
  }

  @Override
  public byte toByte(int index) {
    if (isNull(index)) {
      return 0;
    }
    return getByte(index);
  }

  protected byte getByte(int index) {
    return tinyColumn.getByte(index);
  }

  @Override
  public short toShort(int index) {
    return (short) toByte(index);
  }

  @Override
  public int toInt(int index) {
    return (int) toByte(index);
  }

  @Override
  public long toLong(int index) {
    return (long) toByte(index);
  }

  @Override
  public float toFloat(int index) {
    return toByte(index);
  }

  @Override
  public double toDouble(int index) {
    return toByte(index);
  }

  @Override
  public BigDecimal toBigDecimal(int index) {
    if (isNull(index)) {
      return null;
    }
    return BigDecimal.valueOf(getByte(index), sfScale);
  }

  @Override
  public Object toObject(int index) {
    if (isNull(index)) {
      return null;
    }
    if (!shouldTreatDecimalAsInt()) {
      return toBigDecimal(index);
    }
    return (long) getByte(index);
  }

  @Override
  public String toString(int index) {
    return isNull(index) ? null : Byte.toString(getByte(index));
  }

  @Override
  public boolean toBoolean(int index) {
    byte val = toByte(index);
    if (val == 0) {
      return false;
    } else if (val == 1) {
      return true;
    }
    throw SFSQLException.fromErrorCode(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.BOOLEAN_STR, val);
  }
}
