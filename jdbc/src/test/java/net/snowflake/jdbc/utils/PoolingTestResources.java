package net.snowflake.jdbc.utils;

import java.util.UUID;

/** Shared resource names for pooling integration/e2e tests. */
public final class PoolingTestResources {

  /**
   * Unique per JVM so parallel CI matrix jobs (Java 8/21, json/default) do not clash on the shared
   * Snowflake test account when creating tables or procedures.
   */
  public static final String SUFFIX =
      UUID.randomUUID().toString().replace("-", "").substring(0, 12);

  private PoolingTestResources() {}
}
