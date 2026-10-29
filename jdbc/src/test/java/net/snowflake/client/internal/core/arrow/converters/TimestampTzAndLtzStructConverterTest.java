package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Date;
import java.sql.Time;
import java.sql.Timestamp;
import java.util.HashMap;
import java.util.Map;
import java.util.TimeZone;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.common.core.SnowflakeDateTimeFormat;
import net.snowflake.client.internal.core.arrow.TestHelper;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.Column.PhysicalKind;
import net.snowflake.client.internal.core.arrow.vector.ColumnBatch;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
import net.snowflake.client.internal.jdbc.SnowflakeDateWithTimezone;
import net.snowflake.client.internal.jdbc.SnowflakeTimeWithTimezone;
import net.snowflake.client.internal.jdbc.SnowflakeTimestampWithTimezone;
import org.junit.jupiter.api.Test;

public class TimestampTzAndLtzStructConverterTest extends BaseConverterTest {
  // 2020-01-15T00:00:00Z — post-1582, so Julian→Gregorian adjustTimestamp is a no-op.
  private static final long EPOCH_SECONDS = 1_579_046_400L;
  private static final int FRACTION_NANOS = 123_456_789;
  // Timezone index is biased by 1440 minutes: 1770 - 1440 = +330 minutes = GMT+05:30.
  private static final int IST_TIMEZONE_INDEX = 1770;
  // 1140 - 1440 = -300 minutes = GMT-05:00.
  private static final int EST_TIMEZONE_INDEX = 1140;

  @Test
  public void shouldConvertThreeFieldTimestampTzAndRenderStoredOffset() throws Exception {
    try (ColumnBatch root =
        threeFieldTzRoot(EPOCH_SECONDS, FRACTION_NANOS, IST_TIMEZONE_INDEX, 9)) {
      Column vector = root.column("col");
      ThreeFieldStructToTimestampTZConverter converter =
          new ThreeFieldStructToTimestampTZConverter(vector, 0, this, 9);

      ArrowVectorConverter dispatched = ArrowVectorConverterUtil.initConverter(vector, this, 0);
      assertInstanceOf(ThreeFieldStructToTimestampTZConverter.class, dispatched);

      Timestamp expected = new Timestamp(EPOCH_SECONDS * 1_000L);
      expected.setNanos(FRACTION_NANOS);
      assertEquals(expected, converter.toTimestamp(0, TimeZone.getTimeZone("Europe/Warsaw")));
      assertEquals(expected, converter.toObject(0));
      assertEquals("Wed, 15 Jan 2020 05:30:00 +0530", converter.toString(0));

      Date date = converter.toDate(0, TimeZone.getTimeZone("UTC"), false);
      assertInstanceOf(SnowflakeDateWithTimezone.class, date);
      assertEquals(expected.getTime(), date.getTime());

      Time time = converter.toTime(0);
      assertInstanceOf(SnowflakeTimeWithTimezone.class, time);
      assertEquals(expected.getTime(), time.getTime());
    }
  }

  @Test
  public void shouldReturnSessionZoneWrappersForThreeFieldTimestampTzWhenUseSessionTimezoneEnabled()
      throws Exception {
    try (ColumnBatch root =
        threeFieldTzRoot(EPOCH_SECONDS, FRACTION_NANOS, IST_TIMEZONE_INDEX, 9)) {
      Column vector = root.column("col");
      ThreeFieldStructToTimestampTZConverter converter =
          new ThreeFieldStructToTimestampTZConverter(vector, 0, useSessionTimezoneContext(), 9);

      Timestamp ts = converter.toTimestamp(0, null);
      assertInstanceOf(SnowflakeTimestampWithTimezone.class, ts);
      assertEquals(EPOCH_SECONDS * 1_000L + 123L, ts.getTime());
      assertEquals(FRACTION_NANOS, ts.getNanos());

      Date date = converter.toDate(0, null, false);
      assertEquals("2020-01-15", date.toString());

      Time time = converter.toTime(0);
      assertEquals("05:30:00", time.toString());
    }
  }

  @Test
  public void shouldReadReloadedThreeFieldStructChildrenWithoutRebuildingConverter()
      throws Exception {
    try (ColumnBatch root =
        threeFieldTzRoot(EPOCH_SECONDS, FRACTION_NANOS, IST_TIMEZONE_INDEX, 9)) {
      Column vector = root.column("col");
      ThreeFieldStructToTimestampTZConverter converter =
          new ThreeFieldStructToTimestampTZConverter(vector, 0, this, 9);

      assertEquals("Wed, 15 Jan 2020 05:30:00 +0530", converter.toString(0));

      long reloadedEpoch = EPOCH_SECONDS + 3_600L;
      int reloadedFraction = 987_654_321;
      ColumnBuilder.set(
          child(vector, AbstractArrowVectorConverter.FIELD_NAME_EPOCH), 0, reloadedEpoch);
      ColumnBuilder.set(
          child(vector, AbstractArrowVectorConverter.FIELD_NAME_FRACTION), 0, reloadedFraction);
      ColumnBuilder.set(
          child(vector, AbstractArrowVectorConverter.FIELD_NAME_TIMEZONE), 0, EST_TIMEZONE_INDEX);

      Timestamp expected = new Timestamp(reloadedEpoch * 1_000L);
      expected.setNanos(reloadedFraction);
      assertEquals(expected, converter.toTimestamp(0, null));
      assertEquals("Tue, 14 Jan 2020 20:00:00 -0500", converter.toString(0));
    }
  }

  @Test
  public void shouldHandleNullAndChildNullsAndInvalidConversionsOnThreeFieldTimestampTz()
      throws Exception {
    try (ColumnBatch root =
        threeFieldTzRoot(EPOCH_SECONDS, FRACTION_NANOS, IST_TIMEZONE_INDEX, 9)) {
      Column vector = root.column("col");
      ColumnBuilder.set(
          child(vector, AbstractArrowVectorConverter.FIELD_NAME_EPOCH), 1, EPOCH_SECONDS);
      ColumnBuilder.set(
          child(vector, AbstractArrowVectorConverter.FIELD_NAME_FRACTION), 1, FRACTION_NANOS);
      ColumnBuilder.setNull(child(vector, AbstractArrowVectorConverter.FIELD_NAME_TIMEZONE), 1);
      ColumnBuilder.setDefined(vector, 1);
      ColumnBuilder.setNull(vector, 2);
      ColumnBuilder.setValueCount(vector, 3);
      root.setRowCount(3);

      ThreeFieldStructToTimestampTZConverter converter =
          new ThreeFieldStructToTimestampTZConverter(vector, 0, this, 9);

      assertFalse(converter.isNull(0));
      assertTrue(converter.isNull(1));
      assertTrue(converter.isNull(2));

      assertNull(converter.toTimestamp(1, null));
      assertNull(converter.toObject(1));
      assertNull(converter.toString(1));
      assertNull(converter.toDate(1, null, false));
      assertNull(converter.toTime(1));
      assertFalse(converter.toBoolean(1));
      assertNull(converter.toBytes(1));

      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(0));
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBytes(0));
    }
  }

  @Test
  public void shouldConvertTwoFieldTimestampTzAtScaleZeroAndReloadChildren() throws Exception {
    try (ColumnBatch root = twoFieldTzRoot(EPOCH_SECONDS, IST_TIMEZONE_INDEX, 0)) {
      Column vector = root.column("col");
      TwoFieldStructToTimestampTZConverter converter =
          new TwoFieldStructToTimestampTZConverter(vector, 0, this, 0);

      ArrowVectorConverter dispatched = ArrowVectorConverterUtil.initConverter(vector, this, 0);
      assertInstanceOf(TwoFieldStructToTimestampTZConverter.class, dispatched);

      Timestamp expected = new Timestamp(EPOCH_SECONDS * 1_000L);
      assertEquals(expected, converter.toTimestamp(0, TimeZone.getTimeZone("UTC")));
      assertEquals(expected, converter.toObject(0));
      assertEquals("Wed, 15 Jan 2020 05:30:00 +0530", converter.toString(0));
      assertEquals(new Date(expected.getTime()), converter.toDate(0, null, false));
      assertInstanceOf(SnowflakeTimeWithTimezone.class, converter.toTime(0));

      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(0));
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBytes(0));

      long reloadedEpoch = EPOCH_SECONDS + 7_200L;
      ColumnBuilder.set(
          child(vector, AbstractArrowVectorConverter.FIELD_NAME_EPOCH), 0, reloadedEpoch);
      ColumnBuilder.set(
          child(vector, AbstractArrowVectorConverter.FIELD_NAME_TIMEZONE), 0, EST_TIMEZONE_INDEX);

      assertEquals(new Timestamp(reloadedEpoch * 1_000L), converter.toTimestamp(0, null));
      assertEquals("Tue, 14 Jan 2020 21:00:00 -0500", converter.toString(0));
    }
  }

  @Test
  public void shouldHandleNullsOnTwoFieldTimestampTz() throws Exception {
    try (ColumnBatch root = twoFieldTzRoot(EPOCH_SECONDS, IST_TIMEZONE_INDEX, 0)) {
      Column vector = root.column("col");
      ColumnBuilder.setNull(vector, 1);
      ColumnBuilder.setValueCount(vector, 2);
      root.setRowCount(2);

      TwoFieldStructToTimestampTZConverter converter =
          new TwoFieldStructToTimestampTZConverter(vector, 0, this, 0);

      assertTrue(converter.isNull(1));
      assertNull(converter.toTimestamp(1, null));
      assertNull(converter.toObject(1));
      assertNull(converter.toString(1));
      assertNull(converter.toDate(1, null, false));
      assertNull(converter.toTime(1));
      assertFalse(converter.toBoolean(1));
      assertNull(converter.toBytes(1));
    }
  }

  @Test
  public void shouldConvertTwoFieldTimestampLtzAndReloadChildren() throws Exception {
    try (ColumnBatch root = twoFieldLtzRoot(EPOCH_SECONDS, FRACTION_NANOS, 9)) {
      Column vector = root.column("col");
      TwoFieldStructToTimestampLTZConverter converter =
          new TwoFieldStructToTimestampLTZConverter(vector, 0, this, 9);

      ArrowVectorConverter dispatched = ArrowVectorConverterUtil.initConverter(vector, this, 0);
      assertInstanceOf(TwoFieldStructToTimestampLTZConverter.class, dispatched);

      Timestamp expected = new Timestamp(EPOCH_SECONDS * 1_000L);
      expected.setNanos(FRACTION_NANOS);
      assertEquals(expected, converter.toTimestamp(0, TimeZone.getTimeZone("UTC")));
      assertEquals(expected, converter.toObject(0));
      // Default session timezone in DataConversionContext is America/Los_Angeles (UTC-8 in Jan).
      assertEquals("Tue, 14 Jan 2020 16:00:00 -0800", converter.toString(0));
      assertInstanceOf(SnowflakeDateWithTimezone.class, converter.toDate(0, null, false));
      assertInstanceOf(SnowflakeTimeWithTimezone.class, converter.toTime(0));

      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(0));

      long reloadedEpoch = EPOCH_SECONDS + 86_400L;
      int reloadedFraction = 500_000_000;
      ColumnBuilder.set(
          child(vector, AbstractArrowVectorConverter.FIELD_NAME_EPOCH), 0, reloadedEpoch);
      ColumnBuilder.set(
          child(vector, AbstractArrowVectorConverter.FIELD_NAME_FRACTION), 0, reloadedFraction);

      Timestamp reloaded = new Timestamp(reloadedEpoch * 1_000L);
      reloaded.setNanos(reloadedFraction);
      assertEquals(reloaded, converter.toTimestamp(0, null));
      assertEquals("Wed, 15 Jan 2020 16:00:00 -0800", converter.toString(0));
    }
  }

  @Test
  public void shouldThrowInternalErrorWhenTimestampFormattersAreNull() throws Exception {
    DataConversionContext missingFormatters =
        new DataConversionContext() {
          @Override
          public SnowflakeDateTimeFormat getTimestampTZFormatter() {
            return null;
          }

          @Override
          public SnowflakeDateTimeFormat getTimestampLTZFormatter() {
            return null;
          }
        };

    try (ColumnBatch tz3Root =
            threeFieldTzRoot(EPOCH_SECONDS, FRACTION_NANOS, IST_TIMEZONE_INDEX, 9);
        ColumnBatch tz2Root = twoFieldTzRoot(EPOCH_SECONDS, IST_TIMEZONE_INDEX, 0);
        ColumnBatch ltzRoot = twoFieldLtzRoot(EPOCH_SECONDS, FRACTION_NANOS, 9)) {
      ThreeFieldStructToTimestampTZConverter tz3 =
          new ThreeFieldStructToTimestampTZConverter(
              tz3Root.column("col"), 0, missingFormatters, 9);
      TwoFieldStructToTimestampTZConverter tz2 =
          new TwoFieldStructToTimestampTZConverter(tz2Root.column("col"), 0, missingFormatters, 0);
      TwoFieldStructToTimestampLTZConverter ltz =
          new TwoFieldStructToTimestampLTZConverter(ltzRoot.column("col"), 0, missingFormatters, 9);

      SFSQLException tz3Error = assertThrows(SFSQLException.class, () -> tz3.toString(0));
      assertEquals(ErrorCode.INTERNAL_ERROR, tz3Error.getErrorCode());

      SFSQLException tz2Error = assertThrows(SFSQLException.class, () -> tz2.toString(0));
      assertEquals(ErrorCode.INTERNAL_ERROR, tz2Error.getErrorCode());

      SFSQLException ltzError = assertThrows(SFSQLException.class, () -> ltz.toString(0));
      assertEquals(ErrorCode.INTERNAL_ERROR, ltzError.getErrorCode());
    }
  }

  private static DataConversionContext useSessionTimezoneContext() {
    return new DataConversionContext() {
      @Override
      public boolean isUseSessionTimezone() {
        return true;
      }
    };
  }

  private static Column child(Column struct, String name) {
    return struct.child(name);
  }

  private static ColumnBatch threeFieldTzRoot(
      long epoch, int fraction, int timezoneIndex, int scale) {
    Map<String, String> metadata = timestampMetadata("TIMESTAMP_TZ", scale);
    Column epochs =
        ColumnBuilder.of(AbstractArrowVectorConverter.FIELD_NAME_EPOCH, PhysicalKind.INT64, null);
    Column fractions =
        ColumnBuilder.of(
            AbstractArrowVectorConverter.FIELD_NAME_FRACTION, PhysicalKind.INT32, null);
    Column timezones =
        ColumnBuilder.of(
            AbstractArrowVectorConverter.FIELD_NAME_TIMEZONE, PhysicalKind.INT32, null);
    ColumnBuilder.set(epochs, 0, epoch);
    ColumnBuilder.set(fractions, 0, fraction);
    ColumnBuilder.set(timezones, 0, timezoneIndex);
    return root(metadata, epochs, fractions, timezones);
  }

  private static ColumnBatch twoFieldTzRoot(long epoch, int timezoneIndex, int scale) {
    Map<String, String> metadata = timestampMetadata("TIMESTAMP_TZ", scale);
    Column epochs =
        ColumnBuilder.of(AbstractArrowVectorConverter.FIELD_NAME_EPOCH, PhysicalKind.INT64, null);
    Column timezones =
        ColumnBuilder.of(
            AbstractArrowVectorConverter.FIELD_NAME_TIMEZONE, PhysicalKind.INT32, null);
    ColumnBuilder.set(epochs, 0, epoch);
    ColumnBuilder.set(timezones, 0, timezoneIndex);
    return root(metadata, epochs, timezones);
  }

  private static ColumnBatch twoFieldLtzRoot(long epoch, int fraction, int scale) {
    Map<String, String> metadata = timestampMetadata("TIMESTAMP_LTZ", scale);
    Column epochs =
        ColumnBuilder.of(AbstractArrowVectorConverter.FIELD_NAME_EPOCH, PhysicalKind.INT64, null);
    Column fractions =
        ColumnBuilder.of(
            AbstractArrowVectorConverter.FIELD_NAME_FRACTION, PhysicalKind.INT32, null);
    ColumnBuilder.set(epochs, 0, epoch);
    ColumnBuilder.set(fractions, 0, fraction);
    return root(metadata, epochs, fractions);
  }

  private static Map<String, String> timestampMetadata(String logicalType, int scale) {
    Map<String, String> metadata = new HashMap<>();
    metadata.put("logicalType", logicalType);
    metadata.put("scale", String.valueOf(scale));
    return metadata;
  }

  private static ColumnBatch root(Map<String, String> metadata, Column... children) {
    Column vector = ColumnBuilder.struct("col", metadata, children);
    ColumnBuilder.setDefined(vector, 0);
    ColumnBuilder.setValueCount(vector, 1);
    ColumnBatch batch = ColumnBatch.of(vector);
    batch.setRowCount(1);
    return batch;
  }
}
