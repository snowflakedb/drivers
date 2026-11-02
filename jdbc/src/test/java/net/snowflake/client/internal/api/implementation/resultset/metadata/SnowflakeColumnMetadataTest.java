package net.snowflake.client.internal.api.implementation.resultset.metadata;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.node.ObjectNode;
import java.sql.Types;
import java.util.Collections;
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
  void shouldCopyProtoDimensionWhenPresent() {
    ColumnMetadata col =
        ColumnMetadata.newBuilder().setName("vec").setType("vector").setDimension(128).build();

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals(128, meta.getDimension());
  }

  @Test
  void shouldReportZeroDimensionWhenProtoDimensionIsAbsent() {
    ColumnMetadata col =
        ColumnMetadata.newBuilder()
            .setName("vec")
            .setType("vector")
            .setPrecision(18)
            .setScale(2)
            .setLength(40)
            .build();

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals(0, meta.getDimension());
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
  void shouldReportZeroDimensionWhenJsonCatalogHasNoDimensionFields() {
    ObjectNode col = JsonTestUtils.objectNode();
    col.put("name", "vec_col");
    col.put("type", "vector");
    col.put("precision", 18);
    col.put("scale", 2);
    col.put("length", 40);

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals(0, meta.getDimension());
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
    assertEquals(18, meta.getPrecision());
    assertEquals(0, meta.getScale());
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

  @Test
  void shouldCopyProtoFixedFlagOntoColumnMetadata() {
    ColumnMetadata fixedCol =
        ColumnMetadata.newBuilder()
            .setName("id_col")
            .setType("fixed")
            .setScale(0)
            .setFixed(true)
            .build();
    ColumnMetadata unfixedCol =
        ColumnMetadata.newBuilder()
            .setName("id_col")
            .setType("fixed")
            .setScale(0)
            .setFixed(false)
            .build();

    assertTrue(new SnowflakeColumnMetadata(fixedCol, true).isFixed());
    assertTrue(new SnowflakeColumnMetadata(fixedCol, false).isFixed());
    assertFalse(new SnowflakeColumnMetadata(unfixedCol, true).isFixed());
  }

  @Test
  void shouldMapJsonFixedScaleZeroToBigintWhenTreatDecimalAsIntEnabled() {
    ObjectNode col = JsonTestUtils.objectNode();
    col.put("name", "num_col");
    col.put("type", "fixed");
    col.put("scale", 0);
    col.put("precision", 12);

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, true);

    assertEquals(Types.BIGINT, meta.getType());
    assertEquals(12, meta.getPrecision());
  }

  @Test
  void shouldMapProtoVectorWireTypeToExtraTypesVectorWithDimension() {
    ColumnMetadata col =
        ColumnMetadata.newBuilder()
            .setName("embedding")
            .setType("vector")
            .setDimension(768)
            .build();

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals("VECTOR", meta.getTypeName());
    assertEquals(SnowflakeType.EXTRA_TYPES_VECTOR, meta.getType());
    assertEquals(SnowflakeType.VECTOR, meta.getBase());
    assertEquals(768, meta.getDimension());
  }

  @Test
  void shouldUseVectorDimensionWhenJsonDimensionIsZero() {
    ObjectNode col = JsonTestUtils.objectNode();
    col.put("name", "embedding");
    col.put("type", "vector");
    col.put("dimension", 0);
    col.put("vectorDimension", 256);

    SnowflakeColumnMetadata meta = new SnowflakeColumnMetadata(col, false);

    assertEquals(256, meta.getDimension());
  }

  @Test
  void shouldPopulateAndExposeFieldMetadataProperties() {
    FieldMetadataImpl child = new FieldMetadataImpl();
    child.setName("nested_col");
    child.setTypeName("VARCHAR");
    child.setType(Types.VARCHAR);
    child.setNullable(true);
    child.setByteLength(64);
    child.setPrecision(0);
    child.setScale(0);
    child.setFixed(false);
    child.setBase(SnowflakeType.TEXT);
    child.setFields(Collections.emptyList());

    FieldMetadataImpl parent = new FieldMetadataImpl();
    parent.setName("obj_col");
    parent.setTypeName("OBJECT");
    parent.setType(Types.STRUCT);
    parent.setNullable(false);
    parent.setByteLength(1024);
    parent.setPrecision(18);
    parent.setScale(2);
    parent.setFixed(true);
    parent.setBase(SnowflakeType.OBJECT);
    parent.setFields(Collections.singletonList(child));

    assertEquals("obj_col", parent.getName());
    assertEquals("OBJECT", parent.getTypeName());
    assertEquals(Types.STRUCT, parent.getType());
    assertFalse(parent.isNullable());
    assertEquals(1024, parent.getByteLength());
    assertEquals(18, parent.getPrecision());
    assertEquals(2, parent.getScale());
    assertTrue(parent.isFixed());
    assertEquals(SnowflakeType.OBJECT, parent.getBase());
    assertEquals(1, parent.getFields().size());
    assertEquals("nested_col", parent.getFields().get(0).getName());
    assertEquals(SnowflakeType.TEXT, parent.getFields().get(0).getBase());
  }
}
