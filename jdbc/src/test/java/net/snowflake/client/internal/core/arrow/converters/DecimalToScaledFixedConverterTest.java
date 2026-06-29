package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;

import java.math.BigDecimal;
import java.time.Duration;
import java.util.HashMap;
import java.util.Map;
import net.snowflake.client.internal.core.arrow.TestHelper;
import org.apache.arrow.memory.BufferAllocator;
import org.apache.arrow.memory.RootAllocator;
import org.apache.arrow.vector.DecimalVector;
import org.apache.arrow.vector.types.pojo.ArrowType;
import org.apache.arrow.vector.types.pojo.FieldType;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;

public class DecimalToScaledFixedConverterTest extends BaseConverterTest {
  private static final int PRECISION = 38;
  private static final int SCALE = 0;
  private static final long NANOS_PER_SECOND = 1_000_000_000L;

  private final BufferAllocator allocator = new RootAllocator(Long.MAX_VALUE);

  @AfterEach
  public void closeAllocator() {
    allocator.close();
  }

  private static Map<String, String> fixedFieldMeta() {
    Map<String, String> meta = new HashMap<>();
    meta.put("logicalType", "FIXED");
    meta.put("precision", String.valueOf(PRECISION));
    meta.put("scale", String.valueOf(SCALE));
    return meta;
  }

  /** A FIXED column whose decimal values are nanosecond counts (INTERVAL DAY TO SECOND). */
  private DecimalVector createNanosVector(BigDecimal... nanos) {
    FieldType fieldType =
        new FieldType(true, new ArrowType.Decimal(PRECISION, SCALE, 128), null, fixedFieldMeta());
    DecimalVector vector = new DecimalVector("col_interval", fieldType, allocator);
    for (int i = 0; i < nanos.length; i++) {
      if (nanos[i] == null) {
        vector.setNull(i);
      } else {
        vector.setSafe(i, nanos[i]);
      }
    }
    vector.setValueCount(nanos.length);
    return vector;
  }

  @Test
  public void shouldConvertNanosToDuration() throws Exception {
    // 1.5 seconds = 1_500_000_000 nanos.
    DecimalVector vector = createNanosVector(BigDecimal.valueOf(NANOS_PER_SECOND + 500_000_000L));
    try {
      DecimalToScaledFixedConverter converter = new DecimalToScaledFixedConverter(vector, 0, this);
      assertEquals(Duration.ofSeconds(1, 500_000_000L), converter.toDuration(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldConvertNegativeNanosToNegativeDuration() throws Exception {
    DecimalVector vector =
        createNanosVector(BigDecimal.valueOf(-(NANOS_PER_SECOND + 500_000_000L)));
    try {
      DecimalToScaledFixedConverter converter = new DecimalToScaledFixedConverter(vector, 0, this);
      assertEquals(Duration.ofSeconds(1, 500_000_000L).negated(), converter.toDuration(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldReturnNullDurationForNullIndex() throws Exception {
    DecimalVector vector = createNanosVector((BigDecimal) null);
    try {
      DecimalToScaledFixedConverter converter = new DecimalToScaledFixedConverter(vector, 0, this);
      assertNull(converter.toDuration(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldThrowWhenDurationSecondsOverflowsLong() throws Exception {
    // 1e29 nanos / 1e9 = 1e20 seconds, which does not fit in a long.
    DecimalVector vector = createNanosVector(new BigDecimal("100000000000000000000000000000"));
    try {
      DecimalToScaledFixedConverter converter = new DecimalToScaledFixedConverter(vector, 0, this);
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toDuration(0));
    } finally {
      vector.close();
    }
  }

  @Test
  public void shouldDispatchDecimalVectorToScaledFixedConverter() throws Exception {
    DecimalVector vector = createNanosVector(BigDecimal.valueOf(NANOS_PER_SECOND));
    try {
      ArrowVectorConverter converter = ArrowVectorConverterUtil.initConverter(vector, this, 0);
      assertInstanceOf(DecimalToScaledFixedConverter.class, converter);
      assertEquals(Duration.ofSeconds(1), converter.toDuration(0));
    } finally {
      vector.close();
    }
  }
}
