package net.snowflake.client.internal.api.implementation.connection;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;

import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConfigSetting;
import org.junit.jupiter.api.Test;

public class SnowflakeConnectionImplTest {

  @Test
  public void toConfigSettingMapsLongValuesToInt64() {
    ConfigSetting configSetting = SnowflakeConnectionImpl.toConfigSetting(1234567890123L);

    assertEquals(ConfigSetting.ValueCase.INT_VALUE, configSetting.getValueCase());
    assertEquals(1234567890123L, configSetting.getIntValue());
  }

  @Test
  public void toConfigSettingMapsStringValuesToStringValue() {
    ConfigSetting configSetting = SnowflakeConnectionImpl.toConfigSetting("test-account");

    assertEquals(ConfigSetting.ValueCase.STRING_VALUE, configSetting.getValueCase());
    assertEquals("test-account", configSetting.getStringValue());
  }

  @Test
  public void toConfigSettingMapsBooleanValuesToBoolValue() {
    ConfigSetting configSetting = SnowflakeConnectionImpl.toConfigSetting(Boolean.TRUE);

    assertEquals(ConfigSetting.ValueCase.BOOL_VALUE, configSetting.getValueCase());
    assertEquals(true, configSetting.getBoolValue());
  }

  @Test
  public void toConfigSettingMapsDoubleValuesToDoubleValue() {
    ConfigSetting configSetting = SnowflakeConnectionImpl.toConfigSetting(3.14d);

    assertEquals(ConfigSetting.ValueCase.DOUBLE_VALUE, configSetting.getValueCase());
    assertEquals(3.14d, configSetting.getDoubleValue());
  }

  @Test
  public void toConfigSettingReturnsNullForUnsupportedValues() {
    assertNull(SnowflakeConnectionImpl.toConfigSetting(new Object()));
  }
}
