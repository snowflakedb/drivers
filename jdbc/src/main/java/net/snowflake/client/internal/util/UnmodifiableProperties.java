package net.snowflake.client.internal.util;

import java.util.Map;
import java.util.Properties;
import java.util.function.BiFunction;
import java.util.function.Function;

/**
 * An empty {@link Properties} that rejects every mutation with {@link
 * UnsupportedOperationException}.
 */
public final class UnmodifiableProperties extends Properties {
  private static final long serialVersionUID = 1L;

  private static UnsupportedOperationException unsupported() {
    return new UnsupportedOperationException("These properties are read-only");
  }

  @Override
  public synchronized Object put(Object key, Object value) {
    throw unsupported();
  }

  @Override
  public synchronized Object setProperty(String key, String value) {
    throw unsupported();
  }

  @Override
  public synchronized void putAll(Map<?, ?> t) {
    throw unsupported();
  }

  @Override
  public synchronized Object putIfAbsent(Object key, Object value) {
    throw unsupported();
  }

  @Override
  public synchronized Object remove(Object key) {
    throw unsupported();
  }

  @Override
  public synchronized boolean remove(Object key, Object value) {
    throw unsupported();
  }

  @Override
  public synchronized void clear() {
    throw unsupported();
  }

  @Override
  public synchronized Object replace(Object key, Object value) {
    throw unsupported();
  }

  @Override
  public synchronized boolean replace(Object key, Object oldValue, Object newValue) {
    throw unsupported();
  }

  @Override
  public synchronized void replaceAll(BiFunction<? super Object, ? super Object, ?> function) {
    throw unsupported();
  }

  @Override
  public synchronized Object compute(
      Object key, BiFunction<? super Object, ? super Object, ?> remappingFunction) {
    throw unsupported();
  }

  @Override
  public synchronized Object computeIfAbsent(
      Object key, Function<? super Object, ?> mappingFunction) {
    throw unsupported();
  }

  @Override
  public synchronized Object computeIfPresent(
      Object key, BiFunction<? super Object, ? super Object, ?> remappingFunction) {
    throw unsupported();
  }

  @Override
  public synchronized Object merge(
      Object key, Object value, BiFunction<? super Object, ? super Object, ?> remappingFunction) {
    throw unsupported();
  }
}
