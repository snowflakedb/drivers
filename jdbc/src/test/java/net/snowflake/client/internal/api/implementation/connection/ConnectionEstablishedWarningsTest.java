package net.snowflake.client.internal.api.implementation.connection;

import static net.snowflake.jdbc.utils.TestParameters.props;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;

import java.sql.SQLWarning;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetInfoResponse;
import org.junit.jupiter.api.Test;

class ConnectionEstablishedWarningsTest {

  @Test
  void shouldReturnNullWhenNoPropertiesRequested() {
    SQLWarning warning =
        ConnectionEstablishedWarnings.compute(
            props(), ConnectionGetInfoResponse.newBuilder().setDatabase("SERVER_DB").build());
    assertNull(warning);
  }

  @Test
  void shouldReturnNullWhenAllRequestedPropertiesAreHonoredCaseInsensitively() {
    ConnectionGetInfoResponse info =
        ConnectionGetInfoResponse.newBuilder()
            .setDatabase("TEST_DB")
            .setSchema("TEST_SCHEMA")
            .setRole("TEST_ROLE")
            .setWarehouse("TEST_WH")
            .build();
    SQLWarning warning =
        ConnectionEstablishedWarnings.compute(
            props(
                "database", "test_db",
                "schema", "test_schema",
                "role", "test_role",
                "warehouse", "test_wh"),
            info);
    assertNull(warning);
  }

  @Test
  void shouldWarnWithParityMessageWhenDatabaseDiffers() {
    SQLWarning warning =
        ConnectionEstablishedWarnings.compute(
            props("database", "REQ_DB"),
            ConnectionGetInfoResponse.newBuilder().setDatabase("SERVER_DB").build());
    assertNotNull(warning);
    assertEquals(
        "Connection property value Database is invalid. Value specified by user: REQ_DB,"
            + " returned by server: SERVER_DB.",
        warning.getMessage());
    assertEquals(
        ErrorCode.CONNECTION_ESTABLISHED_WITH_DIFFERENT_PROP.getSqlState(), warning.getSQLState());
    assertEquals(
        ErrorCode.CONNECTION_ESTABLISHED_WITH_DIFFERENT_PROP.getMessageCode(),
        warning.getErrorCode());
    assertNull(warning.getNextWarning());
  }

  @Test
  void shouldChainWarningsInDatabaseSchemaRoleWarehouseOrder() {
    ConnectionGetInfoResponse info =
        ConnectionGetInfoResponse.newBuilder()
            .setDatabase("SERVER_DB")
            .setSchema("SERVER_SCHEMA")
            .setRole("SERVER_ROLE")
            .setWarehouse("SERVER_WH")
            .build();
    SQLWarning database =
        ConnectionEstablishedWarnings.compute(
            props(
                "database", "REQ_DB",
                "schema", "REQ_SCHEMA",
                "role", "REQ_ROLE",
                "warehouse", "REQ_WH"),
            info);
    assertNotNull(database);
    assertEquals("Database", extractProperty(database));
    SQLWarning schema = database.getNextWarning();
    assertNotNull(schema);
    assertEquals("Schema", extractProperty(schema));
    SQLWarning role = schema.getNextWarning();
    assertNotNull(role);
    assertEquals("Role", extractProperty(role));
    SQLWarning warehouse = role.getNextWarning();
    assertNotNull(warehouse);
    assertEquals("Warehouse", extractProperty(warehouse));
    assertNull(warehouse.getNextWarning());
  }

  @Test
  void shouldResolveDbAliasToDatabaseProperty() {
    SQLWarning warning =
        ConnectionEstablishedWarnings.compute(
            props("db", "REQ_DB"),
            ConnectionGetInfoResponse.newBuilder().setDatabase("SERVER_DB").build());
    assertNotNull(warning);
    assertEquals("Database", extractProperty(warning));
    assertEquals(
        ErrorCode.CONNECTION_ESTABLISHED_WITH_DIFFERENT_PROP.getMessageCode(),
        warning.getErrorCode());
  }

  @Test
  void shouldWarnWhenRequestedPropertyHasNoServerCounterpart() {
    SQLWarning warning =
        ConnectionEstablishedWarnings.compute(
            props("warehouse", "REQ_WH"), ConnectionGetInfoResponse.getDefaultInstance());
    assertNotNull(warning);
    assertEquals("Warehouse", extractProperty(warning));
  }

  @Test
  void shouldWarnWhenSessionInfoIsNull() {
    SQLWarning warning = ConnectionEstablishedWarnings.compute(props("database", "REQ_DB"), null);
    assertNotNull(warning);
    assertEquals("Database", extractProperty(warning));
  }

  /** The mismatched property name is the first placeholder in the parity message. */
  private static String extractProperty(SQLWarning warning) {
    String message = warning.getMessage();
    return message.substring("Connection property value ".length(), message.indexOf(" is invalid"));
  }
}
