package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;

import java.sql.Timestamp;
import java.util.HashMap;
import java.util.Map;
import java.util.TimeZone;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBatch;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
import org.junit.jupiter.api.Test;

public class TwoFieldStructToTimestampNTZConverterTest extends BaseConverterTest {
  // 2020-01-15T00:00:00Z — post-1582, so Julian→Gregorian adjustTimestamp is a no-op.
  private static final long EPOCH_SECONDS = 1_579_046_400L;
  private static final int FRACTION_NANOS = 123_456_789;

  @Test
  public void shouldConvertEpochAndFractionToTimestamp() throws Exception {
    try (ColumnBatch root = twoFieldRoot(EPOCH_SECONDS, FRACTION_NANOS)) {
      Column vector = root.column("col");
      TwoFieldStructToTimestampNTZConverter converter =
          new TwoFieldStructToTimestampNTZConverter(vector, 0, noClientTzShift(), 9);

      Timestamp expected = new Timestamp(EPOCH_SECONDS * 1_000L);
      expected.setNanos(FRACTION_NANOS);
      assertEquals(expected, converter.toTimestamp(0, null));
      assertEquals(expected, converter.toObject(0));
      assertEquals("Wed, 15 Jan 2020 00:00:00 Z", converter.toString(0));
    }
  }

  @Test
  public void shouldShiftWallClockWhenHonoringClientTimezone() throws Exception {
    try (ColumnBatch root = twoFieldRoot(EPOCH_SECONDS, FRACTION_NANOS)) {
      Column vector = root.column("col");
      TwoFieldStructToTimestampNTZConverter converter =
          new TwoFieldStructToTimestampNTZConverter(vector, 0, honorClientTz(), 9);

      // 2020-01-15 is PST (UTC-8). moveToTimeZoneOffset(UTC→LA) = 0 - (-8h) = +8h.
      Timestamp expected = new Timestamp(EPOCH_SECONDS * 1_000L + 8L * 3_600_000L);
      expected.setNanos(FRACTION_NANOS);
      assertEquals(expected, converter.toTimestamp(0, TimeZone.getTimeZone("America/Los_Angeles")));
    }
  }

  @Test
  public void shouldReadReloadedStructFieldsWithoutRebuildingConverter() throws Exception {
    try (ColumnBatch root = twoFieldRoot(EPOCH_SECONDS, FRACTION_NANOS)) {
      Column vector = root.column("col");
      TwoFieldStructToTimestampNTZConverter converter =
          new TwoFieldStructToTimestampNTZConverter(vector, 0, noClientTzShift(), 9);

      Timestamp firstBatch = new Timestamp(EPOCH_SECONDS * 1_000L);
      firstBatch.setNanos(FRACTION_NANOS);
      assertEquals(firstBatch, converter.toTimestamp(0, null));

      long reloadedEpoch = EPOCH_SECONDS + 86_400L;
      int reloadedFraction = 987_654_321;
      ColumnBuilder.set(
          vector.child(AbstractArrowVectorConverter.FIELD_NAME_EPOCH), 0, reloadedEpoch);
      ColumnBuilder.set(
          vector.child(AbstractArrowVectorConverter.FIELD_NAME_FRACTION), 0, reloadedFraction);

      Timestamp reloaded = new Timestamp(reloadedEpoch * 1_000L);
      reloaded.setNanos(reloadedFraction);
      assertEquals(
          reloaded,
          converter.toTimestamp(0, null),
          "converter must observe in-place struct reloads on the reused vector");
    }
  }

  @Test
  public void shouldReturnNullForNullStructRow() throws Exception {
    try (ColumnBatch root = twoFieldRoot(EPOCH_SECONDS, FRACTION_NANOS)) {
      Column vector = root.column("col");
      ColumnBuilder.setNull(vector, 1);
      ColumnBuilder.setValueCount(vector, 2);
      root.setRowCount(2);

      TwoFieldStructToTimestampNTZConverter converter =
          new TwoFieldStructToTimestampNTZConverter(vector, 0, noClientTzShift(), 9);

      assertNotNull(converter.toTimestamp(0, null));
      assertNull(converter.toTimestamp(1, null));
      assertNull(converter.toObject(1));
      assertNull(converter.toString(1));
    }
  }

  private ColumnBatch twoFieldRoot(long epoch, int fraction) {
    Map<String, String> metadata = new HashMap<>();
    metadata.put("logicalType", "TIMESTAMP_NTZ");
    Column epochs =
        ColumnBuilder.of(
            AbstractArrowVectorConverter.FIELD_NAME_EPOCH, Column.PhysicalKind.INT64, null);
    Column fractions =
        ColumnBuilder.of(
            AbstractArrowVectorConverter.FIELD_NAME_FRACTION, Column.PhysicalKind.INT32, null);
    ColumnBuilder.set(epochs, 0, epoch);
    ColumnBuilder.set(fractions, 0, fraction);
    Column vector = ColumnBuilder.struct("col", metadata, epochs, fractions);
    ColumnBuilder.setDefined(vector, 0);
    ColumnBuilder.setValueCount(vector, 1);
    ColumnBatch root = ColumnBatch.of(vector);
    root.setRowCount(1);
    return root;
  }

  private static DataConversionContext noClientTzShift() {
    return new DataConversionContext() {
      @Override
      public boolean isHonorClientTZForTimestampNTZ() {
        return false;
      }

      @Override
      public boolean isTreatNTZAsUTC() {
        return true;
      }
    };
  }

  private static DataConversionContext honorClientTz() {
    return new DataConversionContext() {
      @Override
      public boolean isHonorClientTZForTimestampNTZ() {
        return true;
      }

      @Override
      public boolean isTreatNTZAsUTC() {
        return false;
      }

      @Override
      public boolean isUseSessionTimezone() {
        return false;
      }
    };
  }
}
