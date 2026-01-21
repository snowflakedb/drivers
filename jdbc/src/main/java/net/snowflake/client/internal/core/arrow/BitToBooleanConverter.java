package net.snowflake.client.internal.core.arrow;

import net.snowflake.client.jdbc.ErrorCode;
import net.snowflake.client.jdbc.SFException;
import net.snowflake.client.jdbc.SnowflakeType;
import net.snowflake.client.jdbc.SnowflakeUtil;
import org.apache.arrow.vector.BitVector;
import org.apache.arrow.vector.ValueVector;

public class BitToBooleanConverter extends AbstractArrowVectorConverter {
  private final BitVector bitVector;

  public BitToBooleanConverter(
      ValueVector fieldVector, int columnIndex, DataConversionContext context) {
    super(SnowflakeType.BOOLEAN.name(), fieldVector, columnIndex, context);
    this.bitVector = (BitVector) fieldVector;
  }

  @Override
  public boolean toBoolean(int index) {
    return !isNull(index) && bitVector.get(index) == 1;
  }

  @Override
  public Object toObject(int index) {
    return isNull(index) ? null : toBoolean(index);
  }

  @Override
  public String toString(int index) {
    return isNull(index) ? null : Boolean.toString(toBoolean(index));
  }

  @Override
  public byte toByte(int index) throws SFException {
    boolean val = toBoolean(index);
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.BYTE_STR, val);
  }
}
