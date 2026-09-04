package net.snowflake.client.internal.api.implementation.parameters;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Collections;
import java.util.Map;
import net.snowflake.client.internal.unicore.ConfigSettingFactory;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConfigSetting;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

class ParametersRegistryTest {

  private static ParametersRegistry registryOf(Map<String, ConfigSetting> parameters) {
    return new FrozenParametersRegistry(parameters);
  }

  @Test
  void shouldReadNativeBoolValue() {
    ParametersRegistry registry =
        registryOf(
            Collections.singletonMap(
                Parameter.AUTOCOMMIT.getKey(), ConfigSettingFactory.from(true)));
    assertTrue(registry.getBool(Parameter.AUTOCOMMIT));
  }

  @Test
  void shouldReadNativeIntValue() {
    ParametersRegistry registry =
        registryOf(
            Collections.singletonMap(
                Parameter.VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT.getKey(),
                ConfigSettingFactory.from(65536L)));
    assertEquals(65536, registry.getInt(Parameter.VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT));
  }

  @Test
  void shouldFallBackToStringValueForBool() {
    ParametersRegistry registry =
        registryOf(
            Collections.singletonMap(
                Parameter.AUTOCOMMIT.getKey(), ConfigSettingFactory.from("true")));
    assertTrue(registry.getBool(Parameter.AUTOCOMMIT));
  }

  @Test
  void shouldFallBackToStringValueForInt() {
    ParametersRegistry registry =
        registryOf(
            Collections.singletonMap(
                Parameter.VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT.getKey(),
                ConfigSettingFactory.from("65536")));
    assertEquals(65536, registry.getInt(Parameter.VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT));
  }

  @Test
  void shouldDefaultIntWhenStringValueIsNotAnInteger() {
    ParametersRegistry registry =
        registryOf(
            Collections.singletonMap(
                Parameter.VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT.getKey(),
                ConfigSettingFactory.from("not-a-number")));
    assertEquals(
        Integer.parseInt(Parameter.VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT.getDefaultVal()),
        registry.getInt(Parameter.VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT));
  }

  @ParameterizedTest
  @ValueSource(longs = {Integer.MAX_VALUE + 1L, Integer.MIN_VALUE - 1L})
  void shouldDefaultIntWhenNativeValueIsOutsideIntRange(long outOfRange) {
    ParametersRegistry registry =
        registryOf(
            Collections.singletonMap(
                Parameter.VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT.getKey(),
                ConfigSettingFactory.from(outOfRange)));
    assertEquals(
        Integer.parseInt(Parameter.VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT.getDefaultVal()),
        registry.getInt(Parameter.VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT),
        "int64 values outside int range must not wrap");
  }

  @Test
  void shouldReadNativeIntValueAtIntMaxBoundary() {
    ParametersRegistry registry =
        registryOf(
            Collections.singletonMap(
                Parameter.VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT.getKey(),
                ConfigSettingFactory.from((long) Integer.MAX_VALUE)));
    assertEquals(
        Integer.MAX_VALUE, registry.getInt(Parameter.VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT));
  }

  @Test
  void shouldDefaultBoolWhenParameterIsAbsent() {
    ParametersRegistry registry = registryOf(Collections.<String, ConfigSetting>emptyMap());
    assertFalse(registry.getBool(Parameter.AUTOCOMMIT, false));
  }

  @Test
  void shouldDefaultBoolWhenStringValueIsEmpty() {
    ParametersRegistry registry =
        registryOf(
            Collections.singletonMap(Parameter.AUTOCOMMIT.getKey(), ConfigSettingFactory.from("")));
    assertTrue(registry.getBool(Parameter.AUTOCOMMIT));
  }

  @Test
  void shouldDefaultIntWhenValueIsDouble() {
    ParametersRegistry registry =
        registryOf(
            Collections.singletonMap(
                Parameter.VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT.getKey(),
                ConfigSettingFactory.from(65536.0)));
    assertEquals(
        Integer.parseInt(Parameter.VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT.getDefaultVal()),
        registry.getInt(Parameter.VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT));
  }

  @Test
  void shouldDeriveDisplayStringFromNativeBoolValue() {
    ParametersRegistry registry =
        registryOf(
            Collections.singletonMap(
                Parameter.AUTOCOMMIT.getKey(), ConfigSettingFactory.from(true)));
    assertEquals("true", registry.get(Parameter.AUTOCOMMIT));
  }

  @Test
  void shouldDeriveDisplayStringFromNativeIntValue() {
    ParametersRegistry registry =
        registryOf(
            Collections.singletonMap(
                Parameter.VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT.getKey(),
                ConfigSettingFactory.from(65536L)));
    assertEquals("65536", registry.get(Parameter.VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT));
  }

  @Test
  void shouldReturnDefaultStringWhenParameterIsAbsent() {
    ParametersRegistry registry = registryOf(Collections.<String, ConfigSetting>emptyMap());
    assertEquals(
        Parameter.AUTOCOMMIT.getDefaultVal(),
        registry.get(Parameter.AUTOCOMMIT, Parameter.AUTOCOMMIT.getDefaultVal()));
  }
}
