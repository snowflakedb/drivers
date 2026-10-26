package net.snowflake.client.internal.core.arrow.converters;

import java.math.BigDecimal;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.core.arrow.vector.Column;

public class BitToBooleanConverter extends AbstractArrowVectorConverter {
  private final Column bitVector;

  /**
   * @param fieldVector Column
   * @param columnIndex column index
   * @param context DataConversionContext
   */
  public BitToBooleanConverter(Column fieldVector, int columnIndex, DataConversionContext context) {
    super(SnowflakeType.BOOLEAN.name(), fieldVector, columnIndex, context);
    this.bitVector = fieldVector;
  }

  private int getBit(int index) {
    return bitVector.getBit(index);
  }

  @Override
  public boolean toBoolean(int index) {
    if (isNull(index)) {
      return false;
    } else {
      return getBit(index) != 0;
    }
  }

  @Override
  public byte[] toBytes(int index) {
    if (isNull(index)) {
      return null;
    } else if (toBoolean(index)) {

      return new byte[] {1};
    } else {
      return new byte[] {0};
    }
  }

  @Override
  public Object toObject(int index) {
    return isNull(index) ? null : toBoolean(index);
  }

  @Override
  public String toString(int index) {
    return isNull(index) ? null : toBoolean(index) ? "TRUE" : "FALSE";
  }

  @Override
  public short toShort(int rowIndex) {
    return (short) (toBoolean(rowIndex) ? 1 : 0);
  }

  @Override
  public int toInt(int rowIndex) {
    return toBoolean(rowIndex) ? 1 : 0;
  }

  @Override
  public long toLong(int rowIndex) {
    return toBoolean(rowIndex) ? 1 : 0;
  }

  @Override
  public float toFloat(int rowIndex) {
    return toBoolean(rowIndex) ? 1 : 0;
  }

  @Override
  public double toDouble(int rowIndex) {
    return toBoolean(rowIndex) ? 1 : 0;
  }

  @Override
  public BigDecimal toBigDecimal(int rowIndex) {
    return isNull(rowIndex) ? null : toBoolean(rowIndex) ? BigDecimal.ONE : BigDecimal.ZERO;
  }
}
