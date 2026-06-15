package net.snowflake.jdbc.e2e.parity;

import java.sql.Date;
import java.sql.Time;
import java.sql.Timestamp;
import java.time.LocalDate;
import java.time.LocalDateTime;
import java.time.LocalTime;
import java.time.OffsetDateTime;
import java.time.ZoneOffset;
import java.time.format.DateTimeFormatter;
import java.time.format.DateTimeFormatterBuilder;
import java.time.temporal.ChronoField;

/**
 * Materializes the typed Java value to bind for a given (type, literal) cell. Both drivers are fed
 * the same value object so any divergence is in the bind path itself, not in the value we
 * constructed.
 *
 * <p>Every cross-type combination is supported: {@link #asDate} works even for a TIME literal
 * (returns the time-of-day ms anchored at the epoch), so the SET_DATE bind path can be exercised
 * against every column type. Lossy conversions are intentional and identical for both drivers.
 */
final class JavaValues {

  private JavaValues() {}

  private static final DateTimeFormatter NTZ =
      new DateTimeFormatterBuilder()
          .appendPattern("uuuu-MM-dd HH:mm:ss")
          .optionalStart()
          .appendFraction(ChronoField.NANO_OF_SECOND, 1, 9, true)
          .optionalEnd()
          .toFormatter();

  private static final DateTimeFormatter TZ =
      new DateTimeFormatterBuilder()
          .appendPattern("uuuu-MM-dd HH:mm:ss")
          .optionalStart()
          .appendFraction(ChronoField.NANO_OF_SECOND, 1, 9, true)
          .optionalEnd()
          .appendPattern(" XXX")
          .toFormatter();

  private static Snapshot toSnapshot(SfType type, String literal) {
    switch (type) {
      case DATE:
        {
          LocalDate ld = LocalDate.parse(literal);
          long millis = ld.atStartOfDay(ZoneOffset.UTC).toInstant().toEpochMilli();
          return new Snapshot(millis, 0);
        }
      case TIME:
        {
          LocalTime lt = LocalTime.parse(literal);
          long nanosOfDay = lt.toNanoOfDay();
          long millis = nanosOfDay / 1_000_000L;
          int nanos = (int) (nanosOfDay % 1_000_000_000L);
          return new Snapshot(millis, nanos);
        }
      case TIMESTAMP_NTZ:
      case TIMESTAMP_LTZ:
        {
          LocalDateTime ldt = LocalDateTime.parse(literal, NTZ);
          long millis = ldt.toInstant(ZoneOffset.UTC).toEpochMilli();
          int nanos = ldt.getNano();
          return new Snapshot(millis, nanos);
        }
      case TIMESTAMP_TZ:
        {
          OffsetDateTime odt = OffsetDateTime.parse(literal, TZ);
          long millis = odt.toInstant().toEpochMilli();
          int nanos = odt.getNano();
          return new Snapshot(millis, nanos);
        }
      default:
        throw new IllegalStateException("unknown type " + type);
    }
  }

  static Date asDate(SfType type, String literal) {
    return new Date(toSnapshot(type, literal).millis);
  }

  static Time asTime(SfType type, String literal) {
    return new Time(toSnapshot(type, literal).millis);
  }

  static Timestamp asTimestamp(SfType type, String literal) {
    Snapshot s = toSnapshot(type, literal);
    Timestamp ts = new Timestamp(s.millis);
    ts.setNanos(s.nanos);
    return ts;
  }

  static Object asNatural(SfType type, String literal) {
    switch (type) {
      case DATE:
        return asDate(type, literal);
      case TIME:
        return asTime(type, literal);
      default:
        return asTimestamp(type, literal);
    }
  }

  static String asString(String literal) {
    return literal;
  }

  private static final class Snapshot {
    final long millis;
    final int nanos;

    Snapshot(long millis, int nanos) {
      this.millis = millis;
      this.nanos = nanos;
    }
  }
}
