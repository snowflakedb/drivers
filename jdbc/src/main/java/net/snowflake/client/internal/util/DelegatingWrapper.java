package net.snowflake.client.internal.util;

import java.sql.SQLException;
import java.sql.Wrapper;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;

public interface DelegatingWrapper extends Wrapper {

  // These narrow Wrapper's throws-SQLException signatures to an unchecked clause, keeping the
  // checked exception off the impl layer; the decorator boundary reconstructs it from the carrier.
  // The delegate-walking logic is factored into the static resolve* helpers so AbstractDecorator
  // can reuse it without implementing this interface — a decorator must re-expose the checked
  // `throws SQLException`, which Java forbids as a widening override of these narrowed defaults.
  @Override
  default <T> T unwrap(Class<T> iface) {
    return resolveUnwrap(this, getDelegate(), iface);
  }

  @Override
  default boolean isWrapperFor(Class<?> iface) {
    return resolveIsWrapperFor(this, getDelegate(), iface);
  }

  /**
   * Shared {@code unwrap} logic: returns {@code self} or {@code delegate} if either is an {@code
   * iface} instance, else recurses through a foreign {@link Wrapper} delegate, else throws the
   * runtime {@link SFSQLException} carrier.
   */
  static <T> T resolveUnwrap(Object self, Object delegate, Class<T> iface) {
    // check if this object matches
    if (iface.isInstance(self)) {
      return iface.cast(self);
    }

    if (delegate != null) {
      // check if the delegate itself is a direct match
      if (iface.isInstance(delegate)) {
        return iface.cast(delegate);
      }
      // if the delegate is also a Wrapper, unwrap recursively
      if (delegate instanceof Wrapper) {
        try {
          return ((Wrapper) delegate).unwrap(iface);
        } catch (SQLException e) {
          // A DelegatingWrapper delegate is already de-checked; this only fires for a
          // foreign java.sql.Wrapper. Carry it as the runtime type for the boundary.
          throw new SFSQLException(e.getMessage(), e);
        }
      }
    }

    throw new SFSQLException("Cannot unwrap to " + iface.getName());
  }

  // De-checked plain pass-throughs to Wrapper.unwrap/isWrapperFor: no self/delegate instanceof
  // short-circuit (unlike resolve*), so a caller's existing unwrap semantics are preserved.
  static <T> T unwrapUnchecked(Wrapper target, Class<T> iface) {
    try {
      return target.unwrap(iface);
    } catch (SQLException e) {
      throw new SFSQLException(e.getMessage(), e);
    }
  }

  static boolean isWrapperForUnchecked(Wrapper target, Class<?> iface) {
    try {
      return target.isWrapperFor(iface);
    } catch (SQLException e) {
      throw new SFSQLException(e.getMessage(), e);
    }
  }

  /** Shared {@code isWrapperFor} logic; see {@link #resolveUnwrap}. */
  static boolean resolveIsWrapperFor(Object self, Object delegate, Class<?> iface) {
    // check if this object matches
    if (iface.isInstance(self)) {
      return true;
    }

    if (delegate != null) {
      // check if the delegate is a direct match
      if (iface.isInstance(delegate)) {
        return true;
      }
      // if the delegate is also a Wrapper, check recursively
      if (delegate instanceof Wrapper) {
        try {
          return ((Wrapper) delegate).isWrapperFor(iface);
        } catch (SQLException e) {
          throw new SFSQLException(e.getMessage(), e);
        }
      }
    }

    return false;
  }

  /** Provides the underlying object that this wrapper delegates to. */
  default Object getDelegate() {
    return null;
  }
}
