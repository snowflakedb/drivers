package net.snowflake.client.api.pooling;

import javax.sql.ConnectionPoolDataSource;
import net.snowflake.client.api.datasource.SnowflakeDataSource;

public interface SnowflakeConnectionPoolDataSource
    extends ConnectionPoolDataSource, SnowflakeDataSource {}
