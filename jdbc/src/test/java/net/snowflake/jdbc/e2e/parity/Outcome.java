package net.snowflake.jdbc.e2e.parity;

/**
 * Result of a get/set operation: either a normalized value description or an exception class. Used
 * to assert parity even for cases where one or both drivers throw — the test asks whether the two
 * drivers reached the same outcome category.
 *
 * <p>Equality compares only the exception simple-class name (and value description for successes):
 * drivers word the same condition differently (e.g. "Numeric value not recognized" vs "Cannot
 * convert to TIME") so message-level comparison would be brittle. The original message and root
 * cause ARE captured for {@link #toString()} so failure reports show what actually went wrong;
 * they're just excluded from the parity assertion. Tighten later if a real divergence hides behind
 * same-class-different-cause.
 */
final class Outcome {
  final boolean failed;
  /** Used for equality: class name for errors, normalized value description otherwise. */
  final String description;
  /** Extra context (error message + cause chain) included in toString() but NOT in equals(). */
  private final String detail;

  private Outcome(boolean failed, String description, String detail) {
    this.failed = failed;
    this.description = description;
    this.detail = detail;
  }

  static Outcome value(Object v) {
    return new Outcome(false, GetSink.describe(v), null);
  }

  static Outcome error(Throwable t) {
    Throwable root = t;
    while (root.getCause() != null && root.getCause() != root) {
      root = root.getCause();
    }
    return new Outcome(true, "ERR:" + root.getClass().getSimpleName(), describeError(t, root));
  }

  private static String describeError(Throwable thrown, Throwable root) {
    StringBuilder sb = new StringBuilder();
    sb.append(thrown.getClass().getSimpleName());
    String msg = thrown.getMessage();
    if (msg != null && !msg.isEmpty()) {
      sb.append(": ").append(oneLine(msg));
    }
    if (root != thrown) {
      sb.append(" <- ").append(root.getClass().getSimpleName());
      String rootMsg = root.getMessage();
      if (rootMsg != null && !rootMsg.isEmpty()) {
        sb.append(": ").append(oneLine(rootMsg));
      }
    }
    return sb.toString();
  }

  private static String oneLine(String s) {
    return s.replace('\n', ' ').replace('\r', ' ').trim();
  }

  @Override
  public boolean equals(Object o) {
    if (!(o instanceof Outcome)) {
      return false;
    }
    Outcome other = (Outcome) o;
    return failed == other.failed && description.equals(other.description);
  }

  @Override
  public int hashCode() {
    return description.hashCode() * 31 + (failed ? 1 : 0);
  }

  @Override
  public String toString() {
    if (detail == null) {
      return description;
    }
    return description + " (" + detail + ")";
  }
}
