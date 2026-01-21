package net.snowflake.client.internal.core.arrow;

import net.snowflake.client.jdbc.ErrorCode;
import net.snowflake.client.jdbc.SFException;
import net.snowflake.client.jdbc.SnowflakeType;
import net.snowflake.client.jdbc.SnowflakeUtil;
import org.apache.arrow.vector.ValueVector;
import org.apache.arrow.vector.VarBinaryVector;

public class VarBinaryToBinaryConverter extends AbstractArrowVectorConverter {
  private final VarBinaryVector varBinaryVector;

  public VarBinaryToBinaryConverter(
      ValueVector valueVector, int columnIndex, DataConversionContext context) {
    super(SnowflakeType.BINARY.name(), valueVector, columnIndex, context);
    this.varBinaryVector = (VarBinaryVector) valueVector;
  }

  @Override
  public byte[] toBytes(int index) {
    return isNull(index) ? null : varBinaryVector.get(index);
  }

  @Override
  public Object toObject(int index) {
    return toBytes(index);
  }

  @Override
  public String toString(int index) {
    byte[] bytes = toBytes(index);
    return bytes == null ? null : new String(bytes);
  }

  @Override
  public boolean toBoolean(int index) throws SFException {
    String str = toString(index);
    if (str == null) {
      return false;
    }
    throw new SFException(
        ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, SnowflakeUtil.BOOLEAN_STR, str);
  }
}
