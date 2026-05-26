package net.snowflake.client.internal.unicore;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;

import com.google.protobuf.ByteString;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConfigSetting;
import org.junit.jupiter.api.Test;

class ConfigSettingFactoryTest {

  @Test
  void mapsStringToStringValue() {
    ConfigSetting setting = ConfigSettingFactory.from("test-account");

    assertEquals(ConfigSetting.ValueCase.STRING_VALUE, setting.getValueCase());
    assertEquals("test-account", setting.getStringValue());
  }

  @Test
  void mapsLongToIntValue() {
    ConfigSetting setting = ConfigSettingFactory.from(1234567890123L);

    assertEquals(ConfigSetting.ValueCase.INT_VALUE, setting.getValueCase());
    assertEquals(1234567890123L, setting.getIntValue());
  }

  @Test
  void mapsIntegerToIntValue() {
    ConfigSetting setting = ConfigSettingFactory.from(42);

    assertEquals(ConfigSetting.ValueCase.INT_VALUE, setting.getValueCase());
    assertEquals(42L, setting.getIntValue());
  }

  @Test
  void mapsShortToIntValue() {
    ConfigSetting setting = ConfigSettingFactory.from((short) 7);

    assertEquals(ConfigSetting.ValueCase.INT_VALUE, setting.getValueCase());
    assertEquals(7L, setting.getIntValue());
  }

  @Test
  void mapsByteToIntValue() {
    ConfigSetting setting = ConfigSettingFactory.from((byte) 3);

    assertEquals(ConfigSetting.ValueCase.INT_VALUE, setting.getValueCase());
    assertEquals(3L, setting.getIntValue());
  }

  @Test
  void mapsBooleanToBoolValue() {
    ConfigSetting setting = ConfigSettingFactory.from(Boolean.TRUE);

    assertEquals(ConfigSetting.ValueCase.BOOL_VALUE, setting.getValueCase());
    assertEquals(true, setting.getBoolValue());
  }

  @Test
  void mapsDoubleToDoubleValue() {
    ConfigSetting setting = ConfigSettingFactory.from(3.14d);

    assertEquals(ConfigSetting.ValueCase.DOUBLE_VALUE, setting.getValueCase());
    assertEquals(3.14d, setting.getDoubleValue());
  }

  @Test
  void mapsByteArrayToBytesValue() {
    byte[] data = new byte[] {0x01, 0x02, 0x03};
    ConfigSetting setting = ConfigSettingFactory.from(data);

    assertEquals(ConfigSetting.ValueCase.BYTES_VALUE, setting.getValueCase());
    assertEquals(ByteString.copyFrom(data), setting.getBytesValue());
  }

  @Test
  void returnsNullForUnsupportedTypes() {
    assertNull(ConfigSettingFactory.from(new Object()));
  }
}
