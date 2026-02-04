package net.snowflake.client.internal.core.arrow.converters;

import java.math.BigDecimal;
import java.nio.ByteBuffer;
import net.snowflake.client.jdbc.ErrorCode;
import net.snowflake.client.jdbc.SFException;
import net.snowflake.client.jdbc.SnowflakeType;
import net.snowflake.client.jdbc.SnowflakeUtil;
import org.apache.arrow.vector.TinyIntVector;
import org.apache.arrow.vector.ValueVector;

public class TinyIntToFixedConverter extends AbstractArrowVectorConverter {
  protected final TinyIntVector tinyIntVector;
  protected int sfScale;
  protected ByteBuffer byteBuf = ByteBuffer.allocate(TinyIntVector.TYPE_WIDTH);

  public TinyIntToFixedConverter(
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
    this.tinyIntVector = (TinyIntVector) fieldVector;
  }

  @Override
  public byte[] toBytes(int index) throws SFException {
    if (isNull(index)) {
      return null;
    }
    byteBuf.put(0, getByte(index));
    return byteBuf.array();
  }

  @Override
  public byte toByte(int index) {
    return getByte(index);
  }

  protected byte getByte(int index) {
    return tinyIntVector.getDataBuffer().getByte(index * TinyIntVector.TYPE_WIDTH);
  }

  @Override
  public short toShort(int index) throws SFException {
    return (short) toByte(index);
  }

  @Override
  public int toInt(int index) throws SFException {
    return (int) toByte(index);
  }

  @Override
  public long toLong(int index) throws SFException {
    return (long) toByte(index);
  }

  @Override
  public float toFloat(int index) throws SFException {
    return toByte(index);
  }

  @Override
  public double toDouble(int index) throws SFException {
    return toByte(index);
  }

  @Override
  public BigDecimal toBigDecimal(int index) throws SFException {
    if (isNull(index)) {
      return null;
    }
    return BigDecimal.valueOf(getByte(index), sfScale);
  }

  @Override
  public Object toObject(int index) throws SFException {
    if (isNull(index)) {
      return null;
    }
    return (long) getByte(index);
  }

  @Override
  public String toString(int index) throws SFException {
    return isNull(index) ? null : Byte.toString(getByte(index));
  }

  @Override
  public boolean toBoolean(int index) throws SFException {
    byte val = toByte(index);
    if (val == 0) {
      return false;
    } else if (val == 1) {
      return true;
    }
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.BOOLEAN_STR, val);
  }
}
