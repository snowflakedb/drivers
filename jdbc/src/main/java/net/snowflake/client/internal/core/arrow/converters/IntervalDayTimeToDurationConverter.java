package net.snowflake.client.internal.core.arrow.converters;

import java.time.Duration;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.exception.SFException;
import net.snowflake.client.api.resultset.SnowflakeType;
import org.apache.arrow.vector.BigIntVector;
import org.apache.arrow.vector.ValueVector;

/**
 * Converter for the {@code INTERVAL DAY TO TIME} logical type. Snowflake materializes the interval
 * as a single {@code Int64} holding the total number of nanoseconds (signed), which this converter
 * renders as a {@link Duration}. Ported verbatim from snowflake-jdbc's {@code
 * IntervalDayTimeToDurationConverter}.
 */
public class IntervalDayTimeToDurationConverter extends AbstractArrowVectorConverter {

  private final BigIntVector vector;
  private static final long NANOS_IN_SECOND = 1_000_000_000;

  public IntervalDayTimeToDurationConverter(
      ValueVector vector, int columnIndex, DataConversionContext context) {
    super(SnowflakeType.INTERVAL_DAY_TIME.name(), vector, columnIndex, context);
    this.vector = (BigIntVector) vector;
  }

  @Override
  public Duration toDuration(int index) throws SFException {
    if (isNull(index)) {
      return null;
    }
    long numNanos = vector.get(index);
    try {
      int sign = Long.signum(numNanos);
      numNanos = Math.abs(numNanos);
      // Duration.ofSeconds() with a negative second value overflows, so identify the sign of
      // numNanos first and apply Duration.negated() accordingly.
      Duration duration =
          Duration.ofSeconds(numNanos / NANOS_IN_SECOND, numNanos % NANOS_IN_SECOND);
      if (sign >= 0) {
        return duration;
      } else {
        return duration.negated();
      }
    } catch (ArithmeticException e) {
      throw new SFException(ErrorCode.INVALID_VALUE_CONVERT, logicalTypeStr, "Duration", numNanos);
    }
  }

  @Override
  public String toString(int index) throws SFException {
    if (isNull(index)) {
      return null;
    }
    return toDuration(index).toString();
  }

  @Override
  public Object toObject(int index) throws SFException {
    return toDuration(index);
  }
}
