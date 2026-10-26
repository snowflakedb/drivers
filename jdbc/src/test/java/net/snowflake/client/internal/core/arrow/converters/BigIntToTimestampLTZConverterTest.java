package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;

import java.sql.Timestamp;
import java.util.HashMap;
import java.util.Map;
import java.util.TimeZone;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
import org.junit.jupiter.api.Test;

public class BigIntToTimestampLTZConverterTest extends BaseConverterTest {
  // 2020-01-15T00:00:00Z — post-1582, so Julian→Gregorian adjustTimestamp is a no-op.
  private static final long EPOCH_SECONDS = 1_579_046_400L;

  private static Map<String, String> timestampLtzMeta(int scale) {
    Map<String, String> meta = new HashMap<>();
    meta.put("logicalType", "TIMESTAMP_LTZ");
    meta.put("scale", String.valueOf(scale));
    return meta;
  }

  private Column createVector(int scale, long... values) {
    Column vector =
        ColumnBuilder.of("col_ts_ltz", Column.PhysicalKind.INT64, timestampLtzMeta(scale));
    for (int i = 0; i < values.length; i++) {
      ColumnBuilder.set(vector, i, values[i]);
    }
    ColumnBuilder.setValueCount(vector, values.length);
    return vector;
  }

  @Test
  public void shouldConvertScaledEpochToTimestamp() throws Exception {
    int scale = 3;
    // Compact Int64 at scale 3: whole seconds * 10^3 + millisecond fraction.
    long compact = EPOCH_SECONDS * 1_000L + 456L;
    Column vector = createVector(scale, compact);
    try {
      BigIntToTimestampLTZConverter converter =
          new BigIntToTimestampLTZConverter(vector, 0, this, scale);
      Timestamp expected = new Timestamp(EPOCH_SECONDS * 1_000L);
      expected.setNanos(456_000_000); // 456 * 10^(9-3)
      assertEquals(expected, converter.toTimestamp(0, TimeZone.getTimeZone("Europe/Warsaw")));
      assertEquals(expected, converter.toObject(0));
      assertEquals("Tue, 14 Jan 2020 16:00:00 -0800", converter.toString(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldReturnNullForNullRow() throws Exception {
    Column vector = ColumnBuilder.of("col_ts_ltz", Column.PhysicalKind.INT64, timestampLtzMeta(0));
    ColumnBuilder.set(vector, 0, EPOCH_SECONDS);
    ColumnBuilder.setNull(vector, 1);
    ColumnBuilder.setValueCount(vector, 2);
    try {
      BigIntToTimestampLTZConverter converter =
          new BigIntToTimestampLTZConverter(vector, 0, this, 0);
      assertNotNull(converter.toTimestamp(0, null));
      assertNull(converter.toTimestamp(1, null));
      assertNull(converter.toObject(1));
      assertNull(converter.toString(1));
    } finally {
      vector.close();
    }
  }
}
