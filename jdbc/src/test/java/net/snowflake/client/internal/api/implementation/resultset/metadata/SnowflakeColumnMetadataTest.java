package net.snowflake.client.internal.api.implementation.resultset.metadata;

import static org.junit.jupiter.api.Assertions.assertEquals;

import com.fasterxml.jackson.databind.node.ObjectNode;
import java.sql.Types;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ColumnMetadata;
import net.snowflake.jdbc.utils.JsonTestUtils;
import org.junit.jupiter.api.Test;

class SnowflakeColumnMetadataTest {

  @Test
  void shouldMapProtoObjectWireTypeWithGeographyExtToGeographyAndHonorBinaryOutput() {
    ColumnMetadata col =
        ColumnMetadata.newBuilder()
            .setName("geo_col")
            .setType("object")
            .setExtColTypeName("GEOGRAPHY")
            .setUdtOutputType("binary")
            .build();

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals("GEOGRAPHY", meta.getTypeName());
    assertEquals(Types.BINARY, meta.getType());
    assertEquals(SnowflakeType.GEOGRAPHY, meta.getBase());
  }

  @Test
  void shouldMapProtoEmptyExtColTypeNameToWireType() {
    ColumnMetadata col = ColumnMetadata.newBuilder().setName("txt").setType("TEXT").build();

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals("VARCHAR", meta.getTypeName());
    assertEquals(Types.VARCHAR, meta.getType());
    assertEquals(SnowflakeType.TEXT, meta.getBase());
  }

  @Test
  void shouldKeepJsonCatalogWireTypeSeparateFromExtTypeName() {
    ObjectNode col = JsonTestUtils.objectNode();
    col.put("name", "geo_col");
    col.put("type", "object");
    col.put("extTypeName", "GEOGRAPHY");

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals("GEOGRAPHY", meta.getTypeName());
    assertEquals(Types.VARCHAR, meta.getType());
    assertEquals(SnowflakeType.OBJECT, meta.getBase());
  }
}
