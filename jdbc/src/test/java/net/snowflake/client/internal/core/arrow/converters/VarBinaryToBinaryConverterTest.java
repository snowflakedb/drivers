package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.Set;
import java.util.stream.Stream;
import net.snowflake.client.internal.core.arrow.TestHelper;
import net.snowflake.client.internal.util.BinaryOutputFormat;
import net.snowflake.client.internal.util.HexUtil;
import org.apache.arrow.memory.BufferAllocator;
import org.apache.arrow.memory.RootAllocator;
import org.apache.arrow.vector.VarBinaryVector;
import org.apache.arrow.vector.types.Types;
import org.apache.arrow.vector.types.pojo.FieldType;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;

public class VarBinaryToBinaryConverterTest extends BaseConverterTest {
  /** allocator for arrow */
  private final BufferAllocator allocator = new RootAllocator(Long.MAX_VALUE);

  private final Random random = new Random();

  @Test
  public void testConvertToString() {
    final int rowCount = 1000;
    List<byte[]> expectedValues = new ArrayList<>();
    Set<Integer> nullValIndex = new HashSet<>();
    for (int i = 0; i < rowCount; i++) {
      expectedValues.add(TestHelper.randomString(random, 20).getBytes());
    }

    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "BINARY");

    FieldType fieldType =
        new FieldType(true, Types.MinorType.VARBINARY.getType(), null, customFieldMeta);

    VarBinaryVector vector = new VarBinaryVector("col_one", fieldType, allocator);
    for (int i = 0; i < rowCount; i++) {
      boolean isNull = random.nextBoolean();
      if (isNull) {
        vector.setNull(i);
        nullValIndex.add(i);
      } else {
        vector.setSafe(i, expectedValues.get(i));
      }
    }

    ArrowVectorConverter converter = new VarBinaryToBinaryConverter(vector, 0, this);

    for (int i = 0; i < rowCount; i++) {
      String stringVal = converter.toString(i);
      Object objectVal = converter.toObject(i);
      byte[] bytesVal = converter.toBytes(i);
      if (stringVal != null) {
        assertFalse(converter.isNull(i));
      } else {
        assertTrue(converter.isNull(i));
      }

      if (nullValIndex.contains(i)) {
        assertNull(stringVal);
        assertNull(objectVal);
        assertNull(bytesVal);
        assertFalse(converter.toBoolean(i));
      } else {
        assertEquals(HexUtil.bytesToHex(expectedValues.get(i)), stringVal);
        assertArrayEquals(expectedValues.get(i), bytesVal);
        assertArrayEquals(expectedValues.get(i), (byte[]) objectVal);
        int index = i;
        TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(index));
      }
    }
    vector.clear();
  }

  @ParameterizedTest
  @MethodSource("binaryOutputFormatCases")
  public void shouldEncodeStringUsingSessionBinaryOutputFormat(
      BinaryOutputFormat format, byte[] value, String expected) {
    Map<String, String> customFieldMeta = new HashMap<>();
    customFieldMeta.put("logicalType", "BINARY");
    FieldType fieldType =
        new FieldType(true, Types.MinorType.VARBINARY.getType(), null, customFieldMeta);

    try (VarBinaryVector vector = new VarBinaryVector("col_one", fieldType, allocator)) {
      vector.setSafe(0, value);
      DataConversionContext context =
          new DataConversionContext() {
            @Override
            public BinaryOutputFormat getBinaryOutputFormat() {
              return format;
            }
          };

      ArrowVectorConverter converter = new VarBinaryToBinaryConverter(vector, 0, context);

      assertEquals(expected, converter.toString(0));
      assertArrayEquals(value, converter.toBytes(0));
    }
  }

  static Stream<Arguments> binaryOutputFormatCases() {
    byte[] specSample = {
      (byte) 0x01, (byte) 0x23, (byte) 0x45, (byte) 0x67,
      (byte) 0x89, (byte) 0xAB, (byte) 0xCD, (byte) 0xEF
    };
    byte[] unpadded = {(byte) 0xAB, (byte) 0xCD, (byte) 0x12};
    byte[] padded = {(byte) 0x00, (byte) 0xFF, (byte) 0x42, (byte) 0x01};
    byte[] empty = new byte[0];

    return Stream.of(
        Arguments.of(BinaryOutputFormat.HEX, specSample, "0123456789ABCDEF"),
        Arguments.of(BinaryOutputFormat.HEX, unpadded, "ABCD12"),
        Arguments.of(BinaryOutputFormat.HEX, padded, "00FF4201"),
        Arguments.of(BinaryOutputFormat.HEX, empty, ""),
        Arguments.of(BinaryOutputFormat.BASE64, specSample, "ASNFZ4mrze8="),
        Arguments.of(BinaryOutputFormat.BASE64, unpadded, "q80S"),
        Arguments.of(BinaryOutputFormat.BASE64, padded, "AP9CAQ=="),
        Arguments.of(BinaryOutputFormat.BASE64, empty, ""));
  }

  @Test
  public void shouldDefaultToHexWhenSessionParameterIsAbsent() {
    assertEquals(BinaryOutputFormat.HEX, new BaseConverterTest().getBinaryOutputFormat());
  }
}
