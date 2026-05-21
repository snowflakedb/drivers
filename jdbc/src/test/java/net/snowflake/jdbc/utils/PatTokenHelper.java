package net.snowflake.jdbc.utils;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.Random;
import lombok.Getter;

public class PatTokenHelper {

  private final String tokenName;
  @Getter private String tokenSecret;

  public PatTokenHelper() {
    tokenName = "UD_JDBC_E2E_" + String.format("%08x", new Random().nextInt());
  }

  public void create(Connection conn, String user, String role) throws Exception {
    String sql =
        String.format(
            "ALTER USER IF EXISTS %s ADD PROGRAMMATIC ACCESS TOKEN %s ROLE_RESTRICTION = %s",
            user, tokenName, role);
    try (Statement stmt = conn.createStatement();
        ResultSet rs = stmt.executeQuery(sql)) {
      assertTrue(rs.next(), "ALTER USER should return a result");
      tokenSecret = rs.getString(2);
      assertNotNull(tokenSecret, "PAT token secret should not be null");
      assertFalse(tokenSecret.isEmpty(), "PAT token secret should not be empty");
    }
  }

  public void cleanup(Connection conn, String user) throws Exception {
    String sql =
        String.format(
            "ALTER USER IF EXISTS %s REMOVE PROGRAMMATIC ACCESS TOKEN %s", user, tokenName);
    try (Statement stmt = conn.createStatement()) {
      stmt.execute(sql);
    }
  }
}
