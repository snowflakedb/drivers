package net.snowflake.jdbc.utils;

import static net.snowflake.jdbc.utils.TestParameters.buildJdbcUrl;

import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.util.Properties;

public interface WithConnect {
  default Connection connect(Properties props) throws SQLException {
    return DriverManager.getConnection(buildJdbcUrl(props), props);
  }
}
