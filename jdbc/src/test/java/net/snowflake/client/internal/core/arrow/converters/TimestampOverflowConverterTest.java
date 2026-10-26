package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.HashMap;
import java.util.Map;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBatch;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
import org.junit.jupiter.api.Test;

/**
 * Overflow behavior of the fraction-bearing timestamp struct converters ({@code TIMESTAMP_NTZ}
 * two-field, {@code TIMESTAMP_LTZ} two-field, {@code TIMESTAMP_TZ} three-field).
 *
 * <p>When {@code seconds-since-epoch} falls outside the millisecond range a {@code long} can hold,
 * {@link net.snowflake.client.internal.core.arrow.ArrowResultUtil#isTimestampOverflow(long)} trips
 * and — mirroring snowflake-jdbc — {@code toString} falls back to the raw seconds rendered as a
 * {@code BigDecimal}, while {@code getTimestamp}/{@code getObject}/{@code getTime} return {@code
 * null}. {@code getDate} returns {@code null} for LTZ/TZ but throws {@link NullPointerException}
 * for NTZ: the NTZ converter's {@code toDate} does not null-guard the overflow result, and
 * snowflake-jdbc's does not either — the quirk is preserved verbatim for 1:1 parity.
 *
 * <p>These are unit tests rather than {@code DateTimeParityTest} cases on purpose: Snowflake's
 * timestamp domain is year 1–9999, whose epoch seconds sit far inside {@code Long.MAX_VALUE /
 * 1000}, so a live server can never emit an overflowing value and the parity matrices cannot
 * exercise this path. The overflow ports (P1–P3) are therefore verified here by feeding synthetic
 * overflow epochs straight into the struct vectors.
 *
 * <p>Not covered here (by design, matching legacy verbatim): the scale-0 {@code
 * TwoFieldStructToTimestampTZConverter} and the compact {@code BigIntToTimestamp{NTZ,LTZ}Converter}
 * do <b>not</b> guard overflow — neither does snowflake-jdbc — so there is no guarded behavior to
 * assert for them.
 */
public class TimestampOverflowConverterTest extends BaseConverterTest {

  // Positive overflow: Long.MAX_VALUE seconds is > Long.MAX_VALUE / 1000, so isTimestampOverflow
  // trips. The 9-digit nanosecond fraction makes the expected BigDecimal rendering unambiguous.
  private static final long OVERFLOW_EPOCH_SECONDS = Long.MAX_VALUE;
  private static final int OVERFLOW_FRACTION_NANOS = 123456789;
  private static final String EXPECTED_OVERFLOW_STRING = "9223372036854775807.123456789";

  // TIMESTAMP_TZ index is biased by 1440 minutes; 1440 decodes to GMT+00:00. Irrelevant to the
  // overflow path (the stored zone is only read once a non-null Timestamp exists) but must be a
  // valid child value.
  private static final int UTC_TIMEZONE_INDEX = 1440;

  @Test
  public void shouldRenderSecondsBigDecimalAndNullGettersWhenNtzOverflows() throws Exception {
    try (ColumnBatch root =
        twoFieldRoot("TIMESTAMP_NTZ", OVERFLOW_EPOCH_SECONDS, OVERFLOW_FRACTION_NANOS)) {
      Column vector = root.column("col");
      TwoFieldStructToTimestampNTZConverter converter =
          new TwoFieldStructToTimestampNTZConverter(vector, 0, this, 9);

      assertEquals(EXPECTED_OVERFLOW_STRING, converter.toString(0));
      assertNull(converter.toObject(0));
      assertNull(converter.toTimestamp(0, null));
      assertNull(converter.toTime(0));
      // Verbatim legacy quirk: unlike the LTZ/TZ converters, the NTZ converter's toDate does NOT
      // null-guard the overflow result -- snowflake-jdbc's TwoFieldStructToTimestampNTZConverter
      // does `new Date(getTimestamp(...).getTime())` too, so it NPEs identically. Preserved for
      // 1:1 parity; unreachable from a live server (overflow needs year > ~292M, outside
      // Snowflake's year 1-9999 domain). See TIMESTAMP_MIGRATION_PLAN.md "IMPLEMENTED (P5)".
      assertThrows(NullPointerException.class, () -> converter.toDate(0, null, false));
    }
  }

  @Test
  public void shouldRenderSecondsBigDecimalAndNullGettersWhenLtzOverflows() throws Exception {
    try (ColumnBatch root =
        twoFieldRoot("TIMESTAMP_LTZ", OVERFLOW_EPOCH_SECONDS, OVERFLOW_FRACTION_NANOS)) {
      Column vector = root.column("col");
      TwoFieldStructToTimestampLTZConverter converter =
          new TwoFieldStructToTimestampLTZConverter(vector, 0, this, 9);

      assertEquals(EXPECTED_OVERFLOW_STRING, converter.toString(0));
      assertNull(converter.toObject(0));
      assertNull(converter.toTimestamp(0, null));
      assertNull(converter.toDate(0, null, false));
      assertNull(converter.toTime(0));
    }
  }

  @Test
  public void shouldRenderSecondsBigDecimalAndNullGettersWhenTzOverflows() throws Exception {
    try (ColumnBatch root =
        threeFieldRoot(
            "TIMESTAMP_TZ", OVERFLOW_EPOCH_SECONDS, OVERFLOW_FRACTION_NANOS, UTC_TIMEZONE_INDEX)) {
      Column vector = root.column("col");
      ThreeFieldStructToTimestampTZConverter converter =
          new ThreeFieldStructToTimestampTZConverter(vector, 0, this, 9);

      assertEquals(EXPECTED_OVERFLOW_STRING, converter.toString(0));
      assertNull(converter.toObject(0));
      assertNull(converter.toTimestamp(0, null));
      assertNull(converter.toDate(0, null, false));
      assertNull(converter.toTime(0));
    }
  }

  /**
   * Builds a single-row {@code {epoch, fraction}} struct with the given overflow value at index 0.
   */
  private ColumnBatch twoFieldRoot(String logicalType, long epoch, int fraction) {
    return timestampRoot(logicalType, epoch, fraction, null);
  }

  /**
   * Builds a single-row {@code {epoch, fraction, timezone}} struct with the given overflow value at
   * index 0.
   */
  private ColumnBatch threeFieldRoot(
      String logicalType, long epoch, int fraction, int timezoneIndex) {
    return timestampRoot(logicalType, epoch, fraction, timezoneIndex);
  }

  private ColumnBatch timestampRoot(
      String logicalType, long epoch, int fraction, Integer timezoneIndex) {
    Map<String, String> metadata = new HashMap<>();
    metadata.put("logicalType", logicalType);
    Column epochs =
        ColumnBuilder.of(
            AbstractArrowVectorConverter.FIELD_NAME_EPOCH, Column.PhysicalKind.INT64, null);
    Column fractions =
        ColumnBuilder.of(
            AbstractArrowVectorConverter.FIELD_NAME_FRACTION, Column.PhysicalKind.INT32, null);
    ColumnBuilder.set(epochs, 0, epoch);
    ColumnBuilder.set(fractions, 0, fraction);
    Column vector;
    if (timezoneIndex == null) {
      vector = ColumnBuilder.struct("col", metadata, epochs, fractions);
    } else {
      Column timezone =
          ColumnBuilder.of(
              AbstractArrowVectorConverter.FIELD_NAME_TIMEZONE, Column.PhysicalKind.INT32, null);
      ColumnBuilder.set(timezone, 0, timezoneIndex);
      vector = ColumnBuilder.struct("col", metadata, epochs, fractions, timezone);
    }
    ColumnBuilder.setDefined(vector, 0);
    ColumnBuilder.setValueCount(vector, 1);
    ColumnBatch root = ColumnBatch.of(vector);
    root.setRowCount(1);
    return root;
  }
}
