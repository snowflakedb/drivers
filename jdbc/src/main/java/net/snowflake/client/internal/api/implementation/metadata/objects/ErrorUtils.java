package net.snowflake.client.internal.api.implementation.metadata.objects;

import java.sql.SQLException;
import lombok.experimental.UtilityClass;

@UtilityClass
class ErrorUtils {

  // Legacy snowflake-jdbc executeAndReturnEmptyResultIfNotFound() SQL states.
  private static final String SQL_STATE_NO_DATA = "02000";
  private static final String SQL_STATE_BASE_TABLE_OR_VIEW_NOT_FOUND = "42S02";
  private static final String SQL_STATE_SYNTAX_ERROR_OR_ACCESS_RULE_VIOLATION = "42601";

  /** Snowflake vendor code for "Object does not exist, or operation cannot be performed." */
  private static final int OBJECT_DOES_NOT_EXIST_VENDOR_CODE = 2043;

  static boolean isMissingMetadataObject(Throwable error) {
    for (SQLException sqlException = findSQLException(error);
        sqlException != null;
        sqlException = sqlException.getNextException()) {
      if (sqlException.getErrorCode() == OBJECT_DOES_NOT_EXIST_VENDOR_CODE) {
        return true;
      }
      String sqlState = sqlException.getSQLState();
      if (SQL_STATE_NO_DATA.equals(sqlState)
          || SQL_STATE_BASE_TABLE_OR_VIEW_NOT_FOUND.equals(sqlState)) {
        return true;
      }
    }
    return false;
  }

  static boolean isSyntaxError(Throwable error) {
    SQLException sqlException = findSQLException(error);
    return sqlException != null
        && SQL_STATE_SYNTAX_ERROR_OR_ACCESS_RULE_VIOLATION.equals(sqlException.getSQLState());
  }

  private static SQLException findSQLException(Throwable error) {
    Throwable current = error;
    while (current != null) {
      if (current instanceof SQLException) {
        return (SQLException) current;
      }
      current = current.getCause();
    }
    return null;
  }
}
