package net.snowflake.client.internal.log;

/** Used to create SFLogger instance */
public class SFLoggerFactory {

  /**
   * @param clazz Class type that the logger is instantiated
   * @return An SFLogger instance given the name of the class
   */
  public static SFLogger getLogger(Class<?> clazz) {
    return new SLF4JLogger(clazz);
  }

  /**
   * A replacement for getLogger function, whose parameter is Class&lt;?&gt;, when Class&lt;?&gt; is
   * inaccessible. For example, the name we have is an alias name of a class, we can't get the
   * correct Class&lt;?&gt; by the given name.
   *
   * @param name name to indicate the class (might be different with the class name) that the logger
   *     is instantiated
   * @return An SFLogger instance given the name
   */
  public static SFLogger getLogger(String name) {
    return new SLF4JLogger(name);
  }
}
