package net.snowflake.client.internal.core.arrow;

import java.nio.ByteOrder;
import net.snowflake.client.jdbc.ErrorCode;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.BeforeEach;

public class BaseConverterTest implements DataConversionContext {
  protected final int invalidConversionErrorCode = ErrorCode.INVALID_VALUE_CONVERT.getMessageCode();

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
