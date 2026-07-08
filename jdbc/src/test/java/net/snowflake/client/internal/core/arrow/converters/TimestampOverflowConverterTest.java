package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.Arrays;
import java.util.Collections;
import java.util.HashMap;
import java.util.Map;
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
  private final BufferAllocator allocator = new RootAllocator(Long.MAX_VALUE);

  @AfterEach
  public void closeAllocator() {
    allocator.close();
  }

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
    try (VectorSchemaRoot root =
        twoFieldRoot("TIMESTAMP_NTZ", OVERFLOW_EPOCH_SECONDS, OVERFLOW_FRACTION_NANOS)) {
      StructVector vector = (StructVector) root.getVector("col");
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
    try (VectorSchemaRoot root =
        twoFieldRoot("TIMESTAMP_LTZ", OVERFLOW_EPOCH_SECONDS, OVERFLOW_FRACTION_NANOS)) {
      StructVector vector = (StructVector) root.getVector("col");
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
    try (VectorSchemaRoot root =
        threeFieldRoot(
            "TIMESTAMP_TZ", OVERFLOW_EPOCH_SECONDS, OVERFLOW_FRACTION_NANOS, UTC_TIMEZONE_INDEX)) {
      StructVector vector = (StructVector) root.getVector("col");
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
  private VectorSchemaRoot twoFieldRoot(String logicalType, long epoch, int fraction) {
    VectorSchemaRoot root = VectorSchemaRoot.create(twoFieldSchema(logicalType), allocator);
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

  /**
   * Builds a single-row {@code {epoch, fraction, timezone}} struct with the given overflow value at
   * index 0.
   */
  private VectorSchemaRoot threeFieldRoot(
      String logicalType, long epoch, int fraction, int timezoneIndex) {
    VectorSchemaRoot root = VectorSchemaRoot.create(threeFieldSchema(logicalType), allocator);
    StructVector vector = (StructVector) root.getVector("col");
    vector.allocateNew();
    ((BigIntVector) vector.getChild(AbstractArrowVectorConverter.FIELD_NAME_EPOCH))
        .setSafe(0, epoch);
    ((IntVector) vector.getChild(AbstractArrowVectorConverter.FIELD_NAME_FRACTION))
        .setSafe(0, fraction);
    ((IntVector) vector.getChild(AbstractArrowVectorConverter.FIELD_NAME_TIMEZONE))
        .setSafe(0, timezoneIndex);
    vector.setIndexDefined(0);
    vector.setValueCount(1);
    root.setRowCount(1);
    return root;
  }

  private static Schema twoFieldSchema(String logicalType) {
    return structSchema(
        logicalType,
        Arrays.asList(
            signedIntField(AbstractArrowVectorConverter.FIELD_NAME_EPOCH, 64),
            signedIntField(AbstractArrowVectorConverter.FIELD_NAME_FRACTION, 32)));
  }

  private static Schema threeFieldSchema(String logicalType) {
    return structSchema(
        logicalType,
        Arrays.asList(
            signedIntField(AbstractArrowVectorConverter.FIELD_NAME_EPOCH, 64),
            signedIntField(AbstractArrowVectorConverter.FIELD_NAME_FRACTION, 32),
            signedIntField(AbstractArrowVectorConverter.FIELD_NAME_TIMEZONE, 32)));
  }

  private static Schema structSchema(String logicalType, java.util.List<Field> children) {
    Map<String, String> metadata = new HashMap<>();
    metadata.put("logicalType", logicalType);
    Field structField =
        new Field("col", new FieldType(true, ArrowType.Struct.INSTANCE, null, metadata), children);
    return new Schema(Collections.singletonList(structField));
  }

  private static Field signedIntField(String name, int bitWidth) {
    return new Field(
        name, new FieldType(true, new ArrowType.Int(bitWidth, true), null, null), null);
  }
}
