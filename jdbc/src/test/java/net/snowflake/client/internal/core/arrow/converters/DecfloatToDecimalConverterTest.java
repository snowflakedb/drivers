package net.snowflake.client.internal.core.arrow.converters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNull;

import java.math.BigDecimal;
import java.math.BigInteger;
import java.util.HashMap;
import java.util.Map;
import net.snowflake.client.internal.core.arrow.TestHelper;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBatch;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
import org.junit.jupiter.api.Test;

public class DecfloatToDecimalConverterTest extends BaseConverterTest {

  @Test
  public void testDecfloatConversionsAndDispatch() throws Exception {
    Map<String, String> metadata = new HashMap<>();
    metadata.put("logicalType", "DECFLOAT");
    Column significand = ColumnBuilder.of("significand", Column.PhysicalKind.BINARY, null);
    Column exponent = ColumnBuilder.of("exponent", Column.PhysicalKind.INT16, null);
    Column vector = ColumnBuilder.struct("col_one", metadata, significand, exponent);

    try (ColumnBatch root = ColumnBatch.of(vector)) {
      ColumnBuilder.set(significand, 0, new BigInteger("123456").toByteArray());
      ColumnBuilder.set(exponent, 0, (short) -3);
      ColumnBuilder.setDefined(vector, 0);

      ColumnBuilder.setNull(vector, 1);

      ColumnBuilder.set(significand, 2, new BigInteger("2147483648").toByteArray());
      ColumnBuilder.set(exponent, 2, (short) 0);
      ColumnBuilder.setDefined(vector, 2);

      ColumnBuilder.set(significand, 3, new BigInteger("9223372036854775808").toByteArray());
      ColumnBuilder.set(exponent, 3, (short) 0);
      ColumnBuilder.setDefined(vector, 3);

      ColumnBuilder.set(significand, 4, new BigInteger("12345").toByteArray());
      ColumnBuilder.set(exponent, 4, (short) -2);
      ColumnBuilder.setDefined(vector, 4);

      ColumnBuilder.setValueCount(vector, 5);
      root.setRowCount(5);

      ArrowVectorConverter converter = new DecfloatToDecimalConverter(vector, 0, this);
      ArrowVectorConverter dispatchedConverter =
          ArrowVectorConverterUtil.initConverter(vector, this, 0);
      assertInstanceOf(DecfloatToDecimalConverter.class, dispatchedConverter);

      assertEquals(new BigDecimal("123.456"), converter.toBigDecimal(0));
      assertEquals(new BigDecimal("123.456"), converter.toObject(0));
      assertEquals("123.456", converter.toString(0));
      assertEquals(123.456d, converter.toDouble(0), 0.001d);
      assertEquals(123.456f, converter.toFloat(0), 0.001f);

      assertNull(converter.toBigDecimal(1));
      assertNull(converter.toObject(1));
      assertNull(converter.toString(1));
      assertEquals(0d, converter.toDouble(1), 0d);
      assertEquals(0f, converter.toFloat(1), 0f);
      assertEquals(0L, converter.toLong(1));
      assertEquals(0, converter.toInt(1));
      assertEquals((short) 0, converter.toShort(1));

      assertEquals(2147483648L, converter.toLong(2));
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toInt(2));
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toShort(2));
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toLong(3));
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toInt(4));
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toShort(4));
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toLong(4));
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBytes(0));
      TestHelper.assertSFException(invalidConversionErrorCode, () -> converter.toBoolean(0));

      ColumnBuilder.set(significand, 0, new BigInteger("42").toByteArray());
      ColumnBuilder.set(exponent, 0, (short) 0);
      assertEquals(new BigDecimal("42"), converter.toBigDecimal(0));
      assertEquals((short) 42, converter.toShort(0));
      assertEquals(42, converter.toInt(0));
      assertEquals(42L, converter.toLong(0));
    }
  }

  @Test
  public void shouldFormatZeroAndPlainDecfloatWithinThirtyEightCharacters() throws Exception {
    assertDecfloatStrings(
        new BigDecimal[] {
          BigDecimal.ZERO,
          new BigDecimal("123.456"),
          new BigDecimal("1.23E-7"),
          new BigDecimal("-42"),
          new BigDecimal("-1.5"),
          new BigDecimal("-789.012"),
          new BigDecimal(BigInteger.valueOf(-987), 17),
        },
        new String[] {
          "0", "123.456", "0.000000123", "-42", "-1.5", "-789.012", "-0.00000000000000987",
        });
  }

  @Test
  public void shouldFormatWholeNumberDecfloatAsPlainDecimal() throws Exception {
    assertDecfloatStrings(
        new BigDecimal[] {
          new BigDecimal(BigInteger.ONE, -2),
          new BigDecimal(BigInteger.ONE, -6),
          new BigDecimal("100"),
          new BigDecimal("1000000"),
        },
        new String[] {"100", "1000000", "100", "1000000"});
  }

  @Test
  public void shouldFormatDecfloatInNormalizedScientificNotationPastThirtyEightCharacters()
      throws Exception {
    BigDecimal thirtyEightDigitInteger = new BigDecimal(BigInteger.TEN.pow(37));
    BigDecimal thirtyEightSigWithFraction =
        new BigDecimal(BigInteger.TEN.pow(37).add(BigInteger.valueOf(8)), 1);
    assertDecfloatStrings(
        new BigDecimal[] {
          new BigDecimal("1.2e200"),
          new BigDecimal("1E+16384"),
          new BigDecimal("1.2345678901234567890123456789012345678E+100"),
          new BigDecimal("-1.234E+8000"),
          new BigDecimal(BigInteger.valueOf(123), -35),
          new BigDecimal(BigInteger.valueOf(123), -36),
          new BigDecimal(BigInteger.ONE, 36),
          new BigDecimal(BigInteger.ONE, 37),
          thirtyEightDigitInteger,
          thirtyEightSigWithFraction,
        },
        new String[] {
          "1.2e200",
          "1e16384",
          "1.2345678901234567890123456789012345678e100",
          "-1.234e8000",
          "12300000000000000000000000000000000000",
          "1.23e38",
          "0.000000000000000000000000000000000001",
          "1e-37",
          thirtyEightDigitInteger.toPlainString(),
          "1." + zeros(36) + "8e36",
        });
  }

  @Test
  public void shouldFormatDecfloatToStringWithoutChangingNumericValue() throws Exception {
    BigDecimal scientific = new BigDecimal("1.2e200");
    BigDecimal plain = new BigDecimal("123.456");
    BigDecimal hundred = new BigDecimal(BigInteger.ONE, -2);
    BigDecimal million = new BigDecimal(BigInteger.ONE, -6);
    try (DecfloatFixture fixture =
        loadDecfloat(scientific, plain, hundred, million, BigDecimal.ZERO, null)) {
      ArrowVectorConverter converter = fixture.converter;
      assertEquals("1.2e200", converter.toString(0));
      assertEquals("123.456", converter.toString(1));
      assertEquals("100", converter.toString(2));
      assertEquals("1000000", converter.toString(3));
      assertEquals("0", converter.toString(4));
      assertNull(converter.toString(5));
      assertEquals(scientific, converter.toBigDecimal(0));
      assertEquals(plain, converter.toBigDecimal(1));
      assertEquals(0, new BigDecimal("100").compareTo(converter.toBigDecimal(2)));
      assertEquals(0, new BigDecimal("1000000").compareTo(converter.toBigDecimal(3)));
    }
  }

  private void assertDecfloatStrings(BigDecimal[] values, String[] expected) {
    try (DecfloatFixture fixture = loadDecfloat(values)) {
      for (int i = 0; i < expected.length; i++) {
        assertEquals(expected[i], fixture.converter.toString(i));
      }
    }
  }

  private DecfloatFixture loadDecfloat(BigDecimal... values) {
    Map<String, String> metadata = new HashMap<>();
    metadata.put("logicalType", "DECFLOAT");
    Column significand = ColumnBuilder.of("significand", Column.PhysicalKind.BINARY, null);
    Column exponent = ColumnBuilder.of("exponent", Column.PhysicalKind.INT16, null);
    Column vector = ColumnBuilder.struct("col_one", metadata, significand, exponent);
    ColumnBatch root = ColumnBatch.of(vector);
    for (int i = 0; i < values.length; i++) {
      if (values[i] == null) {
        ColumnBuilder.setNull(vector, i);
      } else {
        ColumnBuilder.set(significand, i, values[i].unscaledValue().toByteArray());
        ColumnBuilder.set(exponent, i, (short) -values[i].scale());
        ColumnBuilder.setDefined(vector, i);
      }
    }
    ColumnBuilder.setValueCount(vector, values.length);
    root.setRowCount(values.length);
    return new DecfloatFixture(root, new DecfloatToDecimalConverter(vector, 0, this));
  }

  private static String zeros(int count) {
    StringBuilder out = new StringBuilder(count);
    for (int i = 0; i < count; i++) {
      out.append('0');
    }
    return out.toString();
  }

  private static final class DecfloatFixture implements AutoCloseable {
    private final ColumnBatch batch;
    private final ArrowVectorConverter converter;

    private DecfloatFixture(ColumnBatch batch, ArrowVectorConverter converter) {
      this.batch = batch;
      this.converter = converter;
    }

    @Override
    public void close() {
      batch.close();
    }
  }
}
