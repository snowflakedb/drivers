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
  void shouldMapProtoObjectWireTypeWithGeometryExtToGeometryAndHonorBinaryOutput() {
    ColumnMetadata col =
        ColumnMetadata.newBuilder()
            .setName("geom_col")
            .setType("object")
            .setExtColTypeName("GEOMETRY")
            .setUdtOutputType("binary")
            .build();

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals("GEOMETRY", meta.getTypeName());
    assertEquals(Types.BINARY, meta.getType());
    assertEquals(SnowflakeType.GEOMETRY, meta.getBase());
  }

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

  @Test
  void shouldKeepJsonCatalogWireTypeSeparateFromGeometryExtTypeName() {
    ObjectNode col = JsonTestUtils.objectNode();
    col.put("name", "geom_col");
    col.put("type", "object");
    col.put("extTypeName", "GEOMETRY");

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals("GEOMETRY", meta.getTypeName());
    assertEquals(Types.VARCHAR, meta.getType());
    assertEquals(SnowflakeType.OBJECT, meta.getBase());
  }

  @Test
  void shouldMapProtoGeographyExtToVarcharWhenUdtOutputIsText() {
    ColumnMetadata col =
        ColumnMetadata.newBuilder()
            .setName("geo_col")
            .setType("object")
            .setExtColTypeName("GEOGRAPHY")
            .setUdtOutputType("text")
            .build();

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals("GEOGRAPHY", meta.getTypeName());
    assertEquals(Types.VARCHAR, meta.getType());
    assertEquals(SnowflakeType.GEOGRAPHY, meta.getBase());
  }

  @Test
  void shouldMapProtoGeometryExtToVarcharWhenUdtOutputIsText() {
    ColumnMetadata col =
        ColumnMetadata.newBuilder()
            .setName("geom_col")
            .setType("object")
            .setExtColTypeName("GEOMETRY")
            .setUdtOutputType("text")
            .build();

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals("GEOMETRY", meta.getTypeName());
    assertEquals(Types.VARCHAR, meta.getType());
    assertEquals(SnowflakeType.GEOMETRY, meta.getBase());
  }

  @Test
  void shouldMapProtoBinaryWireGeometryExtToVarcharWhenUdtOutputIsAbsent() {
    ColumnMetadata col =
        ColumnMetadata.newBuilder()
            .setName("geom_col")
            .setType("binary")
            .setExtColTypeName("GEOMETRY")
            .build();

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals("GEOMETRY", meta.getTypeName());
    assertEquals(Types.VARCHAR, meta.getType());
    assertEquals(SnowflakeType.GEOMETRY, meta.getBase());
  }

  @Test
  void shouldMapProtoBinaryWireGeometryExtToBinaryWhenUdtOutputIsBinary() {
    ColumnMetadata col =
        ColumnMetadata.newBuilder()
            .setName("geom_col")
            .setType("binary")
            .setExtColTypeName("GEOMETRY")
            .setUdtOutputType("binary")
            .build();

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals("GEOMETRY", meta.getTypeName());
    assertEquals(Types.BINARY, meta.getType());
    assertEquals(SnowflakeType.GEOMETRY, meta.getBase());
  }
}
