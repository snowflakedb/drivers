package net.snowflake.client.internal.api.implementation.metadata.objects;

import net.snowflake.common.util.Wildcard;

class MetaDataQueryBuilder {

  private final StringBuilder builder = new StringBuilder();
  private final boolean isExactSchema;
  private final boolean useSessionSchema;
  private final boolean enableWildcardsInShowMetadataCommands;
  private boolean earlyExit = false;

  MetaDataQueryBuilder(
      boolean isExactSchema,
      boolean useSessionSchema,
      boolean enableWildcardsInShowMetadataCommands) {
    this.isExactSchema = isExactSchema;
    this.useSessionSchema = useSessionSchema;
    this.enableWildcardsInShowMetadataCommands = enableWildcardsInShowMetadataCommands;
  }

  MetaDataQueryBuilder show(String type) {
    builder.append("show ").append(type);
    return this;
  }

  MetaDataQueryBuilder like(String pattern) {
    if (pattern != null
        && !pattern.isEmpty()
        && !pattern.trim().equals("%")
        && !pattern.trim().equals(".*")) {
      builder.append(" like '").append(escapeSingleQuoteForLikeCommand(pattern)).append("'");
    }
    return this;
  }

  MetaDataQueryBuilder likeWithWildcards(String pattern) {
    builder.append(" like '").append(escapeSingleQuoteForLikeCommand(pattern)).append("'");
    return this;
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

  private boolean isSchemaNameWildcardPattern(String inputString) {
    // if schema contains wildcard, don't treat it as wildcard; treat as just a schema name if
    // session schema or wildcards in identifiers in show metadata queries disabled
    return (useSessionSchema || !enableWildcardsInShowMetadataCommands)
        ? false
        : Wildcard.isWildcardPatternStr(inputString);
  }

  private static String unescapeChars(String escapedString) {
    String unescapedString = escapedString.replace("\\_", "_");
    unescapedString = unescapedString.replace("\\%", "%");
    unescapedString = unescapedString.replace("\\\\", "\\");
    unescapedString = escapeSqlQuotes(unescapedString);
    return unescapedString;
  }

  /** This guards against SQL injections by ensuring that any single quote is escaped properly. */
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
}
