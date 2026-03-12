package net.snowflake.client.api.datasource;

import lombok.AccessLevel;
import lombok.NoArgsConstructor;
import net.snowflake.client.internal.api.implementation.datasource.SnowflakeBasicDataSource;

/**
 * Factory for creating {@link SnowflakeDataSource} instances.
 *
 * <p>This factory provides methods to create different types of Snowflake DataSource
 * implementations. Use this factory instead of directly instantiating DataSource classes.
 */
@NoArgsConstructor(access = AccessLevel.PRIVATE)
public final class SnowflakeDataSourceFactory {

  public static SnowflakeDataSource createDataSource() {
    return new SnowflakeBasicDataSource();
  }
}
