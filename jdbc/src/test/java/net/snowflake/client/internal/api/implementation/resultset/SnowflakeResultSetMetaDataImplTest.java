package net.snowflake.client.internal.api.implementation.resultset;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;

import java.sql.Types;
import org.junit.jupiter.api.Test;

class SnowflakeResultSetMetaDataImplTest {

  @Test
  void getQueryIdReturnsValueProvidedAtConstruction() throws Exception {
    SnowflakeResultSetMetaDataImpl meta =
        new SnowflakeResultSetMetaDataImpl(new String[] {"C"}, new int[] {Types.INTEGER}, "qid-1");
    assertEquals("qid-1", meta.getQueryID());
  }

  @Test
  void getQueryIdReturnsNullWhenConstructedWithoutOne() throws Exception {
    SnowflakeResultSetMetaDataImpl meta =
        new SnowflakeResultSetMetaDataImpl(new String[] {"C"}, new int[] {Types.INTEGER}, null);
    assertNull(meta.getQueryID());
  }
}
