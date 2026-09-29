package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;

import java.sql.Timestamp;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashMap;
import java.util.Map;
import java.util.TimeZone;
import org.apache.arrow.memory.BufferAllocator;
import org.apache.arrow.memory.RootAllocator;
import org.apache.arrow.vector.BigIntVector;
import org.apache.arrow.vector.IntVector;
import org.apache.arrow.vector.VectorSchemaRoot;
import org.apache.arrow.vector.complex.StructVector;
import org.apache.arrow.vector.types.pojo.ArrowType;
import org.apache.arrow.vector.types.pojo.Field;
import org.apache.arrow.vector.types.pojo.FieldType;
import org.apache.arrow.vector.types.pojo.Schema;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;

public class TwoFieldStructToTimestampNTZConverterTest extends BaseConverterTest {
  // 2020-01-15T00:00:00Z — post-1582, so Julian→Gregorian adjustTimestamp is a no-op.
  private static final long EPOCH_SECONDS = 1_579_046_400L;
  private static final int FRACTION_NANOS = 123_456_789;

  private final BufferAllocator allocator = new RootAllocator(Long.MAX_VALUE);

  @AfterEach
  public void closeAllocator() {
    allocator.close();
  }

  @Test
  public void shouldConvertEpochAndFractionToTimestamp() throws Exception {
    try (VectorSchemaRoot root = twoFieldRoot(EPOCH_SECONDS, FRACTION_NANOS)) {
      StructVector vector = (StructVector) root.getVector("col");
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
    try (VectorSchemaRoot root = twoFieldRoot(EPOCH_SECONDS, FRACTION_NANOS)) {
      StructVector vector = (StructVector) root.getVector("col");
      TwoFieldStructToTimestampNTZConverter converter =
          new TwoFieldStructToTimestampNTZConverter(vector, 0, honorClientTz(), 9);

      // 2020-01-15 is PST (UTC-8). moveToTimeZoneOffset(UTC→LA) = 0 - (-8h) = +8h.
      Timestamp expected = new Timestamp(EPOCH_SECONDS * 1_000L + 8L * 3_600_000L);
      expected.setNanos(FRACTION_NANOS);
      assertEquals(expected, converter.toTimestamp(0, TimeZone.getTimeZone("America/Los_Angeles")));
    }
  }

  @Test
  public void shouldReturnNullForNullStructRow() throws Exception {
    try (VectorSchemaRoot root = twoFieldRoot(EPOCH_SECONDS, FRACTION_NANOS)) {
      StructVector vector = (StructVector) root.getVector("col");
      vector.setNull(1);
      vector.setValueCount(2);
      root.setRowCount(2);

      TwoFieldStructToTimestampNTZConverter converter =
          new TwoFieldStructToTimestampNTZConverter(vector, 0, noClientTzShift(), 9);

      assertNotNull(converter.toTimestamp(0, null));
      assertNull(converter.toTimestamp(1, null));
      assertNull(converter.toObject(1));
      assertNull(converter.toString(1));
    }
  }

  private VectorSchemaRoot twoFieldRoot(long epoch, int fraction) {
    VectorSchemaRoot root = VectorSchemaRoot.create(twoFieldSchema(), allocator);
    StructVector vector = (StructVector) root.getVector("col");
    vector.allocateNew();
    ((BigIntVector) vector.getChild(AbstractArrowVectorConverter.FIELD_NAME_EPOCH))
        .setSafe(0, epoch);
    ((IntVector) vector.getChild(AbstractArrowVectorConverter.FIELD_NAME_FRACTION))
        .setSafe(0, fraction);
    vector.setIndexDefined(0);
    vector.setValueCount(1);
    root.setRowCount(1);
    return root;
  }

  private static Schema twoFieldSchema() {
    Map<String, String> metadata = new HashMap<>();
    metadata.put("logicalType", "TIMESTAMP_NTZ");
    Field structField =
        new Field(
            "col",
            new FieldType(true, ArrowType.Struct.INSTANCE, null, metadata),
            Arrays.asList(
                signedIntField(AbstractArrowVectorConverter.FIELD_NAME_EPOCH, 64),
                signedIntField(AbstractArrowVectorConverter.FIELD_NAME_FRACTION, 32)));
    return new Schema(Collections.singletonList(structField));
  }

  private static Field signedIntField(String name, int bitWidth) {
    return new Field(
        name, new FieldType(true, new ArrowType.Int(bitWidth, true), null, null), null);
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
