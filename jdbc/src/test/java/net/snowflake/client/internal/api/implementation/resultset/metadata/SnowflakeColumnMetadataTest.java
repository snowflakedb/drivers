package net.snowflake.client.internal.api.implementation.resultset.metadata;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

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
  void shouldKeepJsonCatalogObjectWireTypeOnVarcharWhenGeometryExtRequestsBinaryOutput() {
    ObjectNode col = JsonTestUtils.objectNode();
    col.put("name", "geom_col");
    col.put("type", "object");
    col.put("extTypeName", "GEOMETRY");
    col.put("outputType", "binary");

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals("GEOMETRY", meta.getTypeName());
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

  @Test
  void shouldMapProtoBinaryWireGeographyExtToVarcharWhenUdtOutputIsAbsent() {
    ColumnMetadata col =
        ColumnMetadata.newBuilder()
            .setName("geo_col")
            .setType("binary")
            .setExtColTypeName("GEOGRAPHY")
            .build();

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals("GEOGRAPHY", meta.getTypeName());
    assertEquals(Types.VARCHAR, meta.getType());
    assertEquals(SnowflakeType.GEOGRAPHY, meta.getBase());
  }

  @Test
  void shouldMapProtoBinaryWireGeographyExtToBinaryWhenUdtOutputIsBinary() {
    ColumnMetadata col =
        ColumnMetadata.newBuilder()
            .setName("geo_col")
            .setType("binary")
            .setExtColTypeName("GEOGRAPHY")
            .setUdtOutputType("binary")
            .build();

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals("GEOGRAPHY", meta.getTypeName());
    assertEquals(Types.BINARY, meta.getType());
    assertEquals(SnowflakeType.GEOGRAPHY, meta.getBase());
  }

  @Test
  void shouldReadVectorDimensionFromJsonCatalogWhenDimensionIsAbsent() {
    ObjectNode col = JsonTestUtils.objectNode();
    col.put("name", "vec_col");
    col.put("type", "vector");
    col.put("vectorDimension", 384);

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals(384, meta.getDimension());
  }

  @Test
  void shouldPreferDimensionOverVectorDimensionOnJsonCatalog() {
    ObjectNode col = JsonTestUtils.objectNode();
    col.put("name", "vec_col");
    col.put("type", "vector");
    col.put("dimension", 128);
    col.put("vectorDimension", 384);

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals(128, meta.getDimension());
  }

  @Test
  void shouldKeepJsonCatalogObjectWireTypeWhenOutputTypeIsBinary() {
    ObjectNode col = JsonTestUtils.objectNode();
    col.put("name", "geo_col");
    col.put("type", "object");
    col.put("extTypeName", "GEOGRAPHY");
    col.put("outputType", "binary");

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals("GEOGRAPHY", meta.getTypeName());
    assertEquals(Types.VARCHAR, meta.getType());
    assertEquals(SnowflakeType.OBJECT, meta.getBase());
  }

  @Test
  void shouldCopyProtoColumnSourceAndAutoIncrementFlags() {
    ColumnMetadata col =
        ColumnMetadata.newBuilder()
            .setName("id_col")
            .setType("fixed")
            .setColumnSrcDatabase("DB1")
            .setColumnSrcSchema("SCH1")
            .setColumnSrcTable("TBL1")
            .setIsAutoIncrement(true)
            .build();

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals("DB1", meta.getColumnSrcDatabase());
    assertEquals("SCH1", meta.getColumnSrcSchema());
    assertEquals("TBL1", meta.getColumnSrcTable());
    assertTrue(meta.isAutoIncrement());
  }

  @Test
  void shouldCopyJsonCatalogSourceAndAutoIncrementFlags() {
    ObjectNode col = JsonTestUtils.objectNode();
    col.put("name", "id_col");
    col.put("type", "fixed");
    col.put("database", "DB1");
    col.put("schema", "SCH1");
    col.put("table", "TBL1");
    col.put("isAutoIncrement", true);

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals("DB1", meta.getColumnSrcDatabase());
    assertEquals("SCH1", meta.getColumnSrcSchema());
    assertEquals("TBL1", meta.getColumnSrcTable());
    assertTrue(meta.isAutoIncrement());
  }

  @Test
  void shouldDefaultJsonCatalogAutoIncrementToFalseWhenAbsent() {
    ObjectNode col = JsonTestUtils.objectNode();
    col.put("name", "txt");
    col.put("type", "text");

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertFalse(meta.isAutoIncrement());
  }

  @Test
  void shouldReportBigintForScaleZeroFixedWhenTreatDecimalAsIntTrue() {
    ColumnMetadata col =
        ColumnMetadata.newBuilder()
            .setName("num_col")
            .setType("fixed")
            .setScale(0)
            .setPrecision(18)
            .build();

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, true);

    assertEquals(Types.BIGINT, meta.getType());
    assertEquals("NUMBER", meta.getTypeName());
    assertEquals(SnowflakeType.FIXED, meta.getBase());
  }

  @Test
  void shouldReportDecimalForScaleZeroFixedWhenTreatDecimalAsIntFalse() {
    ColumnMetadata col =
        ColumnMetadata.newBuilder()
            .setName("num_col")
            .setType("fixed")
            .setScale(0)
            .setPrecision(18)
            .build();

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals(Types.DECIMAL, meta.getType());
    assertEquals(SnowflakeType.FIXED, meta.getBase());
  }

  @Test
  void shouldKeepDecimalTypeForNonZeroScaleRegardlessOfTreatDecimalAsInt() {
    ColumnMetadata col =
        ColumnMetadata.newBuilder()
            .setName("num_col")
            .setType("fixed")
            .setScale(2)
            .setPrecision(18)
            .build();

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, true);

    assertEquals(Types.DECIMAL, meta.getType());
  }

  @Test
  void shouldHonorTreatDecimalAsIntOnJsonCatalogFixedColumn() {
    ObjectNode col = JsonTestUtils.objectNode();
    col.put("name", "num_col");
    col.put("type", "fixed");
    col.put("scale", 0);
    col.put("precision", 10);

    SnowflakeColumnMetadata asInt = new SnowflakeColumnMetadata(col, true);
    SnowflakeColumnMetadata asDecimal = new SnowflakeColumnMetadata(col, false);

    assertEquals(Types.BIGINT, asInt.getType());
    assertEquals(Types.DECIMAL, asDecimal.getType());
  }
}
