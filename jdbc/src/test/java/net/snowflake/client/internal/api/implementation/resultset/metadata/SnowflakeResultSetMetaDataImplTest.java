package net.snowflake.client.internal.api.implementation.resultset.metadata;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;

import java.util.Collections;
import net.snowflake.client.internal.core.arrow.converters.DataConversionContext;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ColumnMetadata;
import org.junit.jupiter.api.Test;

class SnowflakeResultSetMetaDataImplTest {

  @Test
  void shouldGetQueryIdReturnsValueProvidedAtConstruction() throws Exception {
    SnowflakeResultSetMetaDataImpl meta =
        SnowflakeResultSetMetaDataImpl.from(
            "qid-1",
            Collections.singletonList(ColumnMetadata.getDefaultInstance()),
            new DataConversionContext() {});
    assertEquals("qid-1", meta.getQueryID());
  }

  @Test
  void shouldGetQueryIdReturnsNullWhenConstructedWithoutOne() throws Exception {
    SnowflakeResultSetMetaDataImpl meta =
        SnowflakeResultSetMetaDataImpl.from(
            null,
            Collections.singletonList(ColumnMetadata.getDefaultInstance()),
            new DataConversionContext() {});
    assertNull(meta.getQueryID());
  }

  @Test
  void shouldReturnsViewWithAsyncQueryIdWithoutMutatingSync() throws Exception {
    SnowflakeResultSetMetaDataImpl sync =
        SnowflakeResultSetMetaDataImpl.from(
            "result-scan-id",
            Collections.singletonList(ColumnMetadata.getDefaultInstance()),
            new DataConversionContext() {});

    SnowflakeResultSetMetaDataImpl async =
        SnowflakeResultSetMetaDataImpl.toAsync(sync, "original-async-id");

    assertEquals("original-async-id", async.getQueryID());
    assertEquals("result-scan-id", sync.getQueryID());
  }
}
