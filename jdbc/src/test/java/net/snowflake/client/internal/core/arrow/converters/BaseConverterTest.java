package net.snowflake.client.internal.core.arrow.converters;

import java.nio.ByteOrder;
import java.util.HashMap;
import java.util.Map;
import net.snowflake.client.api.exception.ErrorCode;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.BeforeEach;

public class BaseConverterTest implements DataConversionContext {
  protected final int invalidConversionErrorCode = ErrorCode.INVALID_VALUE_CONVERT.getMessageCode();

  protected static Map<String, String> fixedMeta(int precision, int scale) {
    Map<String, String> meta = new HashMap<>();
    meta.put("logicalType", "FIXED");
    meta.put("precision", String.valueOf(precision));
    meta.put("scale", String.valueOf(scale));
    return meta;
  }

  protected static DataConversionContext decimalAsBigDecimalContext() {
    return new DataConversionContext() {
      @Override
      public boolean isTreatDecimalAsInt() {
        return false;
      }

      @Override
      public boolean isArrowTreatDecimalAsInt() {
        return false;
      }
    };
  }

  @AfterEach
  public void clearTimeZone() {
    System.clearProperty("user.timezone");
  }

  @BeforeEach
  public void assumeLittleEndian() {
    Assumptions.assumeTrue(
        ByteOrder.nativeOrder().equals(ByteOrder.LITTLE_ENDIAN),
        "Arrow doesn't support cross endianness");
  }
}
