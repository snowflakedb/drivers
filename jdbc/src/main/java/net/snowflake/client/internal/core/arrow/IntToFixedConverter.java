package net.snowflake.client.internal.core.arrow;

import java.math.BigDecimal;
import java.nio.ByteBuffer;
import net.snowflake.client.jdbc.ErrorCode;
import net.snowflake.client.jdbc.SFException;
import net.snowflake.client.jdbc.SnowflakeType;
import net.snowflake.client.jdbc.SnowflakeUtil;
import org.apache.arrow.vector.IntVector;
import org.apache.arrow.vector.ValueVector;

public class IntToFixedConverter extends AbstractArrowVectorConverter {
  protected final IntVector intVector;
  protected int sfScale;
  protected ByteBuffer byteBuf = ByteBuffer.allocate(IntVector.TYPE_WIDTH);

  public IntToFixedConverter(
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
    this.intVector = (IntVector) fieldVector;
  }

  @Override
  public byte[] toBytes(int index) throws SFException {
    if (isNull(index)) {
      return null;
    }
    byteBuf.putInt(0, getInt(index));
    return byteBuf.array();
  }

  @Override
  public byte toByte(int index) throws SFException {
    int intVal = toInt(index);
    byte byteVal = (byte) intVal;
    if (byteVal == intVal) {
      return byteVal;
    }
    throw new SFException(ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, "byte", intVal);
  }

  @Override
  public short toShort(int index) throws SFException {
    int intVal = toInt(index);
    short shortVal = (short) intVal;
    if (shortVal == intVal) {
      return shortVal;
    }
    throw new SFException(ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, "short", intVal);
  }

  protected int getInt(int index) {
    return intVector.getDataBuffer().getInt(index * IntVector.TYPE_WIDTH);
  }

  @Override
  public int toInt(int index) throws SFException {
    if (intVector.isNull(index)) {
      return 0;
    }
    return getInt(index);
  }

  @Override
  public long toLong(int index) throws SFException {
    return (long) toInt(index);
  }

  @Override
  public float toFloat(int index) throws SFException {
    return toInt(index);
  }

  @Override
  public double toDouble(int index) throws SFException {
    return toInt(index);
  }

  @Override
  public BigDecimal toBigDecimal(int index) throws SFException {
    if (intVector.isNull(index)) {
      return null;
    }
    return BigDecimal.valueOf((long) getInt(index), sfScale);
  }

  @Override
  public Object toObject(int index) throws SFException {
    if (isNull(index)) {
      return null;
    }
    return (long) getInt(index);
  }

  @Override
  public String toString(int index) throws SFException {
    return isNull(index) ? null : Integer.toString(getInt(index));
  }

  @Override
  public boolean toBoolean(int index) throws SFException {
    int val = toInt(index);
    if (val == 0) {
      return false;
    } else if (val == 1) {
      return true;
    }
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.BOOLEAN_STR, val);
  }
}
