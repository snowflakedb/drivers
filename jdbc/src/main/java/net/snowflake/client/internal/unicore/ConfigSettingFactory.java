package net.snowflake.client.internal.unicore;

import com.google.protobuf.ByteString;
import java.security.PrivateKey;
import java.util.Base64;
import lombok.AccessLevel;
import lombok.NoArgsConstructor;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConfigSetting;

@NoArgsConstructor(access = AccessLevel.PRIVATE)
public class ConfigSettingFactory {

  /**
   * Converts a Java value to a protobuf {@link ConfigSetting}. Returns {@code null} if the value
   * type is not supported.
   */
  public static ConfigSetting from(Object value) {
    if (value instanceof String) {
      return ConfigSetting.newBuilder().setStringValue((String) value).build();
    }
    if (value instanceof Byte
        || value instanceof Short
        || value instanceof Integer
        || value instanceof Long) {
      return ConfigSetting.newBuilder().setIntValue(((Number) value).longValue()).build();
    }
    if (value instanceof Boolean) {
      return ConfigSetting.newBuilder().setBoolValue((Boolean) value).build();
    }
    if (value instanceof Double) {
      return ConfigSetting.newBuilder().setDoubleValue((Double) value).build();
    }
    if (value instanceof byte[]) {
      return ConfigSetting.newBuilder().setBytesValue(ByteString.copyFrom((byte[]) value)).build();
    }
    if (value instanceof PrivateKey) {
      String base64 = Base64.getEncoder().encodeToString(((PrivateKey) value).getEncoded());
      return ConfigSetting.newBuilder().setStringValue(base64).build();
    }
    return null;
  }
}
