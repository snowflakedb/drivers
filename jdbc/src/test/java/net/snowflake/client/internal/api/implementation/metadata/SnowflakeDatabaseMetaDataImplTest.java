package net.snowflake.client.internal.api.implementation.metadata;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.mockito.Mockito.mock;

import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

class SnowflakeDatabaseMetaDataImplTest {

  private SnowflakeConnectionImpl connection;
  private SnowflakeDatabaseMetaDataImpl metadata;

  @BeforeEach
  void setUp() {
    connection = mock(SnowflakeConnectionImpl.class);
    metadata = new SnowflakeDatabaseMetaDataImpl(connection);
  }

  @Test
  void getDriverNameReturnsCanonicalDriverName() throws Exception {
    assertEquals(SnowflakeDriver.DRIVER_NAME, metadata.getDriverName());
    assertEquals("Snowflake JDBC Driver", metadata.getDriverName());
  }

  @Test
  void getDriverVersionReturnsConstant() throws Exception {
    assertEquals(SnowflakeDriver.DRIVER_VERSION, metadata.getDriverVersion());
  }

  @Test
  void getDriverMajorMinorReturnConstants() {
    assertEquals(SnowflakeDriver.MAJOR_VERSION, metadata.getDriverMajorVersion());
    assertEquals(SnowflakeDriver.MINOR_VERSION, metadata.getDriverMinorVersion());
  }

  @Test
  void getJDBCMajorMinorReturnFourTwo() throws Exception {
    assertEquals(4, metadata.getJDBCMajorVersion());
    assertEquals(2, metadata.getJDBCMinorVersion());
  }
}
