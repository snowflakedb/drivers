package net.snowflake.client.internal.core.arrow.converters;

import java.time.Period;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.core.arrow.vector.Column;

/**
 * Converter for the {@code INTERVAL YEAR TO MONTH} logical type. Snowflake materializes the
 * interval as a signed integer holding the total number of months. The physical width is int8,
 * int16, int32, or int64. The total-months value is split into a {@link Period} of years and months
 * (days are always 0).
 */
public class IntervalYearMonthToPeriodConverter extends AbstractArrowVectorConverter {

  private final Column column;
  private static final int MONTHS_IN_YEAR = 12;

  public IntervalYearMonthToPeriodConverter(
      Column vector, int columnIndex, DataConversionContext context) {
    super(SnowflakeType.INTERVAL_YEAR_MONTH.name(), vector, columnIndex, context);
    this.column = vector;
  }

  @Override
  public Period toPeriod(int index) {
    if (isNull(index)) {
      return null;
    }
    long value = column.getSigned(index);
    return Period.of((int) (value / MONTHS_IN_YEAR), (int) (value % MONTHS_IN_YEAR), 0);
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
