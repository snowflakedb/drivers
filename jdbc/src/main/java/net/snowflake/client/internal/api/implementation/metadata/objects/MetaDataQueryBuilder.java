package net.snowflake.client.internal.api.implementation.metadata.objects;

import static net.snowflake.client.internal.api.implementation.metadata.objects.MatchingUtils.isPatternMatchingAll;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import lombok.AccessLevel;
import lombok.RequiredArgsConstructor;
import net.snowflake.common.util.Wildcard;

@RequiredArgsConstructor(access = AccessLevel.PACKAGE)
class MetaDataQueryBuilder {

  private final StringBuilder builder = new StringBuilder();
  private final boolean isExactSchema;
  private final boolean useSessionSchema;
  private final boolean enableWildcardsInShowMetadataCommands;
  private boolean earlyExit = false;

  MetaDataQueryBuilder show(String type) {
    builder.append("show ").append(type);
    return this;
  }

  MetaDataQueryBuilder like(String pattern) {
    if (!isPatternMatchingAll(pattern)) {
      builder.append(" like '").append(escapeSingleQuoteForLikeCommand(pattern)).append("'");
    }
    return this;
  }

  MetaDataQueryBuilder likeSchema(String schemaPattern) {
    if (isExactSchema && enableWildcardsInShowMetadataCommands && schemaPattern != null) {
      String escapedSchemaPattern =
          schemaPattern.replaceAll("_", "\\\\\\\\_").replaceAll("%", "\\\\\\\\%");
      builder
          .append(" like '")
          .append(escapeSingleQuoteForLikeCommand(escapedSchemaPattern))
          .append("'");
      return this;
    }
    return like(schemaPattern);
  }

  MetaDataQueryBuilder inAccount() {
    return in(null, null);
  }

  MetaDataQueryBuilder in(String catalog) {
    return in(catalog, null);
  }

  MetaDataQueryBuilder in(String catalog, String schema) {
    return in(catalog, schema, null);
  }

  MetaDataQueryBuilder in(String catalog, String schema, String table) {
    if (catalog == null) {
      builder.append(" in account");
    } else if (catalog.isEmpty()) {
      earlyExit = true;
    } else {
      String catalogEscaped = escapeSqlQuotes(catalog);
      if (schema == null || isSchemaNameWildcardPattern(schema)) {
        builder.append(" in database \"").append(catalogEscaped).append("\"");
      } else if (schema.isEmpty()) {
        earlyExit = true;
      } else {
        String schemaUnescaped = isExactSchema ? schema : unescapeChars(schema);
        if (table == null
            || (Wildcard.isWildcardPatternStr(table) && enableWildcardsInShowMetadataCommands)) {
          builder
              .append(" in schema \"")
              .append(catalogEscaped)
              .append("\".\"")
              .append(schemaUnescaped)
              .append("\"");
        } else if (table.isEmpty()) {
          earlyExit = true;
        } else {
          String tableNameUnescaped = unescapeChars(table);
          builder
              .append(" in table \"")
              .append(catalogEscaped)
              .append("\".\"")
              .append(schemaUnescaped)
              .append("\".\"")
              .append(tableNameUnescaped)
              .append("\"");
        }
      }
    }
    return this;
  }

  String build() {
    if (earlyExit) {
      return null;
    }
    return builder.toString();
  }

  TablePrivilegesQuery showTablePrivileges(String catalog, String schema, String table) {
    String sqlQuery =
        "select table_catalog, table_schema, table_name, grantor, grantee, privilege_type,"
            + " is_grantable from ";

    if (catalog != null
        && !catalog.isEmpty()
        && !catalog.trim().equals("%")
        && !catalog.trim().equals(".*")) {
      sqlQuery += "\"" + escapeSqlQuotes(catalog) + "\".";
    }
    sqlQuery += "information_schema.table_privileges";

    List<String> parameters = new ArrayList<>(2);
    boolean hasTablePredicate = false;
    if (!isPatternMatchingAll(table)) {
      sqlQuery += " where table_name = ?";
      parameters.add(table);
      hasTablePredicate = true;
    }

    if (!isPatternMatchingAll(schema)) {
      String schemaValue = isExactSchema ? schema : unescapeWildcardChars(schema);
      if (hasTablePredicate) {
        sqlQuery += " and table_schema = ?";
      } else {
        sqlQuery += " where table_schema = ?";
      }
      parameters.add(schemaValue);
    }
    sqlQuery += " order by table_catalog, table_schema, table_name, privilege_type";
    return new TablePrivilegesQuery(sqlQuery, parameters);
  }

  private boolean isSchemaNameWildcardPattern(String inputString) {
    // if schema contains wildcard, don't treat it as wildcard; treat as just a schema name if
    // session schema or wildcards in identifiers in show metadata queries disabled
    return (useSessionSchema || !enableWildcardsInShowMetadataCommands)
        ? false
        : Wildcard.isWildcardPatternStr(inputString);
  }

  private static String unescapeChars(String escapedString) {
    return escapeSqlQuotes(unescapeWildcardChars(escapedString));
  }

  private static String unescapeWildcardChars(String escapedString) {
    String unescapedString = escapedString.replace("\\_", "_");
    unescapedString = unescapedString.replace("\\%", "%");
    unescapedString = unescapedString.replace("\\\\", "\\");
    return unescapedString;
  }

  /** Ensures that any single quote is escaped properly for embedding in a LIKE literal. */
  private static String escapeSingleQuoteForLikeCommand(String arg) {
    if (arg == null) {
      return null;
    }
    int i = 0;
    int index = arg.indexOf("'", i);
    while (index != -1) {
      if (index == 0 || (index > 0 && arg.charAt(index - 1) != '\\')) {
        arg = arg.replace("'", "\\'");
        i = index + 2;
      } else {
        i = index + 1;
      }
      index = i < arg.length() ? arg.indexOf("'", i) : -1;
    }
    return arg;
  }

  // In SQL, double quotes must be escaped with an additional pair of double quotes.
  // Add additional quotes to avoid syntax errors with SQL queries.
  private static String escapeSqlQuotes(String originalString) {
    return originalString.replace("\"", "\"\"");
  }

  static final class TablePrivilegesQuery {
    private final String sql;
    private final List<String> parameters;

    TablePrivilegesQuery(String sql, List<String> parameters) {
      this.sql = sql;
      this.parameters = Collections.unmodifiableList(new ArrayList<>(parameters));
    }

    String sql() {
      return sql;
    }

    List<String> parameters() {
      return parameters;
    }
  }
}
