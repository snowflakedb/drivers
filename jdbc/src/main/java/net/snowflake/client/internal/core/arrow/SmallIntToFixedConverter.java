package net.snowflake.client.internal.core.arrow;

import java.math.BigDecimal;
import java.nio.ByteBuffer;
import net.snowflake.client.jdbc.ErrorCode;
import net.snowflake.client.jdbc.SFException;
import net.snowflake.client.jdbc.SnowflakeType;
import net.snowflake.client.jdbc.SnowflakeUtil;
import org.apache.arrow.vector.SmallIntVector;
import org.apache.arrow.vector.ValueVector;

public class SmallIntToFixedConverter extends AbstractArrowVectorConverter {
  protected final SmallIntVector smallIntVector;
  protected int sfScale;
  protected ByteBuffer byteBuf = ByteBuffer.allocate(SmallIntVector.TYPE_WIDTH);

  public SmallIntToFixedConverter(
      ValueVector fieldVector, int columnIndex, DataConversionContext context) {
    super(
        String.format(
            "%s(%s,%s)",
            SnowflakeType.FIXED,
            fieldVector.getField().getMetadata().get("precision"),
            fieldVector.getField().getMetadata().get("scale")),
        fieldVector,
        columnIndex,
        context);
    this.smallIntVector = (SmallIntVector) fieldVector;
  }

  @Override
  public byte[] toBytes(int index) throws SFException {
    if (isNull(index)) {
      return null;
    }
    byteBuf.putShort(0, getShort(index));
    return byteBuf.array();
  }

  protected short getShort(int index) {
    return smallIntVector.getDataBuffer().getShort(index * SmallIntVector.TYPE_WIDTH);
  }

  @Override
  public byte toByte(int index) throws SFException {
    short shortVal = toShort(index);
    byte byteVal = (byte) shortVal;
    if (byteVal == shortVal) {
      return byteVal;
    }
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.BYTE_STR, shortVal);
  }

  @Override
  public short toShort(int index) throws SFException {
    if (smallIntVector.isNull(index)) {
      return 0;
    }
    return getShort(index);
  }

  @Override
  public int toInt(int index) throws SFException {
    return (int) toShort(index);
  }

  @Override
  public long toLong(int index) throws SFException {
    return (long) toShort(index);
  }

  @Override
  public float toFloat(int index) throws SFException {
    return toShort(index);
  }

  @Override
  public double toDouble(int index) throws SFException {
    return toShort(index);
  }

  @Override
  public BigDecimal toBigDecimal(int index) throws SFException {
    if (smallIntVector.isNull(index)) {
      return null;
    }
    return BigDecimal.valueOf(getShort(index), sfScale);
  }

  @Override
  public Object toObject(int index) throws SFException {
    if (smallIntVector.isNull(index)) {
      return null;
    }
    return (long) getShort(index);
  }

  @Override
  public String toString(int index) throws SFException {
    return isNull(index) ? null : Short.toString(getShort(index));
  }

  @Override
  public boolean toBoolean(int index) throws SFException {
    short val = toShort(index);
    if (val == 0) {
      return false;
    } else if (val == 1) {
      return true;
    }
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.BOOLEAN_STR, val);
  }
}
