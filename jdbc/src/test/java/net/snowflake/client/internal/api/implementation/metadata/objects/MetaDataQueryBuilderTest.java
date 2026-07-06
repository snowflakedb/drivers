package net.snowflake.client.internal.api.implementation.metadata.objects;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;

import org.junit.jupiter.api.Test;

class MetaDataQueryBuilderTest {

  // --- show() ---

  @Test
  void shouldBuildBasicShowQuery() {
    String result = builder().show("databases").build();
    assertEquals("show databases", result);
  }

  // --- like() ---

  @Test
  void shouldOmitLikeClauseWhenPatternIsNull() {
    String result = builder().show("schemas").like(null).build();
    assertEquals("show schemas", result);
  }

  @Test
  void shouldOmitLikeClauseWhenPatternIsEmpty() {
    String result = builder().show("schemas").like("").build();
    assertEquals("show schemas", result);
  }

  @Test
  void shouldOmitLikeClauseWhenPatternIsPercentOnly() {
    String result = builder().show("schemas").like("%").build();
    assertEquals("show schemas", result);
  }

  @Test
  void shouldOmitLikeClauseWhenPatternIsPercent_WithSpaces() {
    String result = builder().show("schemas").like("  %  ").build();
    assertEquals("show schemas", result);
  }

  @Test
  void shouldOmitLikeClauseWhenPatternIsDotStarOnly() {
    String result = builder().show("schemas").like(".*").build();
    assertEquals("show schemas", result);
  }

  @Test
  void shouldAppendLikeClauseForNonWildcardPattern() {
    String result = builder().show("schemas").like("MY_SCHEMA").build();
    assertEquals("show schemas like 'MY_SCHEMA'", result);
  }

  @Test
  void shouldEscapeSingleQuoteInLikePattern() {
    String result = builder().show("schemas").like("it's").build();
    assertEquals("show schemas like 'it\\'s'", result);
  }

  // --- likeWithWildcards() ---

  @Test
  void shouldEscapeSingleQuoteInLikeSchema() {
    String result = builder().show("schemas").likeSchema("O'BRIEN").build();
    assertEquals("show schemas like 'O\\'BRIEN'", result);
  }

  @Test
  void shouldEscapeSingleQuoteInLikeSchemaWhenExactSchemaAndWildcardsEnabled() {
    MetaDataQueryBuilder qb = new MetaDataQueryBuilder(true, false, true);
    String result = qb.show("schemas").likeSchema("O'BRIEN").build();
    assertEquals("show schemas like 'O\\'BRIEN'", result);
  }

  @Test
  void shouldEscapeWildcardsAndSingleQuoteInLikeSchemaWhenExactSchema() {
    MetaDataQueryBuilder qb = new MetaDataQueryBuilder(true, false, true);
    String result = qb.show("schemas").likeSchema("O'_%").build();
    assertEquals("show schemas like 'O\\'\\\\_\\\\%'", result);
  }

  // --- inAccount() / in(null) ---

  @Test
  void shouldAppendInAccountWhenCatalogIsNull() {
    String result = builder().show("databases").inAccount().build();
    assertEquals("show databases in account", result);
  }

  @Test
  void shouldAppendInAccountWhenCallingInWithNullCatalog() {
    String result = builder().show("schemas").in((String) null).build();
    assertEquals("show schemas in account", result);
  }

  // --- in(catalog) — empty catalog triggers early exit ---

  @Test
  void shouldReturnNullWhenCatalogIsEmpty() {
    String result = builder().show("schemas").in("").build();
    assertNull(result);
  }

  // --- in(catalog) — real catalog ---

  @Test
  void shouldAppendInDatabaseForNonNullCatalog() {
    String result = builder().show("schemas").in("MY_DB").build();
    assertEquals("show schemas in database \"MY_DB\"", result);
  }

  @Test
  void shouldEscapeDoubleQuoteInCatalogName() {
    String result = builder().show("schemas").in("DB\"NAME").build();
    assertEquals("show schemas in database \"DB\"\"NAME\"", result);
  }

  // --- in(catalog, schema) — null schema → in database ---

  @Test
  void shouldAppendInDatabaseWhenSchemaIsNull() {
    String result = builder().show("tables").in("MY_DB", null).build();
    assertEquals("show tables in database \"MY_DB\"", result);
  }

  // --- in(catalog, schema) — empty schema → early exit ---

  @Test
  void shouldReturnNullWhenSchemaIsEmpty() {
    String result = builder().show("tables").in("MY_DB", "").build();
    assertNull(result);
  }

  // --- in(catalog, schema) — wildcard schema with wildcards enabled ---

  @Test
  void shouldAppendInDatabaseWhenSchemaIsWildcardAndWildcardsEnabled() {
    MetaDataQueryBuilder qb = new MetaDataQueryBuilder(false, false, true);
    // "%" is a wildcard pattern recognised by Wildcard.isWildcardPatternStr
    String result = qb.show("tables").in("MY_DB", "%").build();
    assertEquals("show tables in database \"MY_DB\"", result);
  }

  // --- in(catalog, schema) — wildcard schema with wildcards disabled → treated as literal name ---

  @Test
  void shouldAppendInSchemaWhenSchemaIsWildcardButWildcardsDisabled() {
    MetaDataQueryBuilder qb = new MetaDataQueryBuilder(false, false, false);
    String result = qb.show("tables").in("MY_DB", "MY_%SCHEMA").build();
    // wildcards disabled → schema treated as literal; unescapeChars converts \_ → _, \% → %
    assertEquals("show tables in schema \"MY_DB\".\"MY_%SCHEMA\"", result);
  }

  // --- in(catalog, schema) — useSessionSchema true → wildcard not applied ---

  @Test
  void shouldAppendInSchemaWhenUseSessionSchemaIsTrue() {
    MetaDataQueryBuilder qb = new MetaDataQueryBuilder(false, true, true);
    // useSessionSchema=true suppresses wildcard detection → schema is literal
    String result = qb.show("tables").in("MY_DB", "PUBLIC").build();
    assertEquals("show tables in schema \"MY_DB\".\"PUBLIC\"", result);
  }

  // --- in(catalog, schema) — isExactSchema true → no unescaping ---

  @Test
  void shouldNotUnescapeSchemaWhenIsExactSchema() {
    MetaDataQueryBuilder qb = new MetaDataQueryBuilder(true, false, false);
    String result = qb.show("tables").in("MY_DB", "MY\\_SCHEMA").build();
    assertEquals("show tables in schema \"MY_DB\".\"MY\\_SCHEMA\"", result);
  }

  // --- in(catalog, schema) — isExactSchema false → unescape \_ and \% ---

  @Test
  void shouldUnescapeUnderscoreInSchemaWhenNotExactSchema() {
    MetaDataQueryBuilder qb = new MetaDataQueryBuilder(false, false, false);
    String result = qb.show("tables").in("MY_DB", "MY\\_SCHEMA").build();
    assertEquals("show tables in schema \"MY_DB\".\"MY_SCHEMA\"", result);
  }

  @Test
  void shouldUnescapePercentInSchemaWhenNotExactSchema() {
    MetaDataQueryBuilder qb = new MetaDataQueryBuilder(false, false, false);
    String result = qb.show("tables").in("MY_DB", "SCHEMA\\%NAME").build();
    assertEquals("show tables in schema \"MY_DB\".\"SCHEMA%NAME\"", result);
  }

  @Test
  void shouldUnescapeBackslashInSchemaWhenNotExactSchema() {
    MetaDataQueryBuilder qb = new MetaDataQueryBuilder(false, false, false);
    String result = qb.show("tables").in("MY_DB", "SCHEMA\\\\NAME").build();
    assertEquals("show tables in schema \"MY_DB\".\"SCHEMA\\NAME\"", result);
  }

  @Test
  void shouldEscapeDoubleQuoteInSchemaNameWhenNotExactSchema() {
    MetaDataQueryBuilder qb = new MetaDataQueryBuilder(false, false, false);
    String result = qb.show("tables").in("MY_DB", "SC\"HEMA").build();
    assertEquals("show tables in schema \"MY_DB\".\"SC\"\"HEMA\"", result);
  }

  // --- chaining show → like → in ---

  @Test
  void shouldCombineShowLikeAndInDatabase() {
    String result = builder().show("schemas").like("PUBLIC").in("MY_DB").build();
    assertEquals("show schemas like 'PUBLIC' in database \"MY_DB\"", result);
  }

  @Test
  void shouldCombineShowLikeAndInSchema() {
    String result =
        new MetaDataQueryBuilder(false, false, false)
            .show("tables")
            .like("MY_TABLE")
            .in("MY_DB", "PUBLIC")
            .build();
    assertEquals("show tables like 'MY_TABLE' in schema \"MY_DB\".\"PUBLIC\"", result);
  }

  // --- build() returns null on early exit, not empty string ---

  @Test
  void shouldReturnNullNotEmptyStringOnEarlyExit() {
    assertNull(builder().show("schemas").in("").build());
    assertNull(builder().show("schemas").in("MY_DB", "").build());
  }

  // --- helpers ---

  private static MetaDataQueryBuilder builder() {
    return new MetaDataQueryBuilder(false, false, true);
  }
}
