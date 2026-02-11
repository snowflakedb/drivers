package net.snowflake.client.internal.log;

public class SFLoggerFactory {

  /**
   * @param clazz Class type that the logger is instantiated
   * @return An SFLogger instance given the name of the class
   */
  public static SFLogger getLogger(Class<?> clazz) {
    return new SLF4JLogger(clazz);
  }

  /**
   * @param name name to indicate the class that the logger is instantiated
   * @return An SFLogger instance given the name
   */
  public static SFLogger getLogger(String name) {
    return new SLF4JLogger(name);
  }
}
