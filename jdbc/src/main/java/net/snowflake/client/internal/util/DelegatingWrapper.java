package net.snowflake.client.internal.util;

import java.sql.SQLException;
import java.sql.Wrapper;

public interface DelegatingWrapper extends Wrapper {

  @Override
  default <T> T unwrap(Class<T> iface) throws SQLException {
    // check if this object matches
    if (iface.isInstance(this)) {
      return iface.cast(this);
    }

    // get the underlying delegate
    Object delegate = getDelegate();

    if (delegate != null) {
      // check if the delegate itself is a direct match
      if (iface.isInstance(delegate)) {
        return iface.cast(delegate);
      }
      // if the delegate is also a Wrapper, unwrap recursively
      if (delegate instanceof Wrapper) {
        return ((Wrapper) delegate).unwrap(iface);
      }
    }

    throw new SQLException("Cannot unwrap to " + iface.getName());
  }

  @Override
  default boolean isWrapperFor(Class<?> iface) throws SQLException {
    // check if this object matches
    if (iface.isInstance(this)) {
      return true;
    }

    //
    // get the underlying delegate
    Object delegate = getDelegate();

    if (delegate != null) {
      // check if the delegate is a direct match
      if (iface.isInstance(delegate)) {
        return true;
      }
      // if the delegate is also a Wrapper, check recursively
      if (delegate instanceof Wrapper) {
        return ((Wrapper) delegate).isWrapperFor(iface);
      }
    }

    return false;
  }

  /** Provides the underlying object that this wrapper delegates to. */
  default Object getDelegate() {
    return null;
  }
}
