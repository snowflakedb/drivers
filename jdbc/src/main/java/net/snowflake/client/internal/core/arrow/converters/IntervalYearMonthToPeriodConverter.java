package net.snowflake.client.internal.core.arrow.converters;

import java.time.Period;
import net.snowflake.client.api.resultset.SnowflakeType;
import org.apache.arrow.vector.BigIntVector;
import org.apache.arrow.vector.IntVector;
import org.apache.arrow.vector.SmallIntVector;
import org.apache.arrow.vector.ValueVector;

/**
 * Converter for the {@code INTERVAL YEAR TO MONTH} logical type. Snowflake materializes the
 * interval as a signed integer holding the total number of months; the physical Arrow width depends
 * on the range (SB2 → {@link SmallIntVector}, SB4 → {@link IntVector}, SB8 → {@link BigIntVector}).
 * The total-months value is split into a {@link Period} of years and months (days are always 0).
 * Ported verbatim from snowflake-jdbc's {@code IntervalYearMonthToPeriodConverter}.
 */
public class IntervalYearMonthToPeriodConverter extends AbstractArrowVectorConverter {

  private SmallIntVector smallIntVector;
  private IntVector intVector;
  private BigIntVector bigIntVector;
  private static final int MONTHS_IN_YEAR = 12;

  public IntervalYearMonthToPeriodConverter(
      ValueVector vector, int columnIndex, DataConversionContext context) {
    super(SnowflakeType.INTERVAL_YEAR_MONTH.name(), vector, columnIndex, context);
    if (vector instanceof SmallIntVector) {
      // Underlying Interval Year-Month type is SB2
      this.smallIntVector = (SmallIntVector) vector;
    } else if (vector instanceof IntVector) {
      // Underlying Interval Year-Month type is SB4
      this.intVector = (IntVector) vector;
    } else if (vector instanceof BigIntVector) {
      // Underlying Interval Year-Month type is SB8
      this.bigIntVector = (BigIntVector) vector;
    }
  }

  @Override
  public Period toPeriod(int index) {
    if (isNull(index)) {
      return null;
    }
    if (smallIntVector != null) {
      short value = smallIntVector.get(index);
      return Period.of(value / MONTHS_IN_YEAR, value % MONTHS_IN_YEAR, 0);
    } else if (intVector != null) {
      int value = intVector.get(index);
      return Period.of(value / MONTHS_IN_YEAR, value % MONTHS_IN_YEAR, 0);
    } else {
      long value = bigIntVector.get(index);
      return Period.of((int) (value / MONTHS_IN_YEAR), (int) (value % MONTHS_IN_YEAR), 0);
    }
  }

  @Override
  public String toString(int index) {
    if (isNull(index)) {
      return null;
    }
    return toPeriod(index).toString();
  }

  @Override
  public Object toObject(int index) {
    return toPeriod(index);
  }
}
