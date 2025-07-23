package com.snowflake.jdbc;

import org.junit.Test;
import static org.junit.Assert.*;

import java.sql.*;

/**
 * Basic tests for the Snowflake JDBC Driver
 */
public class SnowflakeDriverTest {
    
    @Test
    public void testDriverRegistration() throws SQLException {
        // Test that the driver is properly registered
        Driver driver = DriverManager.getDriver("jdbc:snowflake://test.snowflakecomputing.com");
        assertNotNull("Driver should be registered", driver);
        assertTrue("Driver should be instance of SnowflakeDriver", 
                  driver instanceof SnowflakeDriver);
    }
    
    @Test
    public void testAcceptsURL() throws SQLException {
        SnowflakeDriver driver = new SnowflakeDriver();
        
        // Test valid URLs
        assertTrue("Should accept snowflake URL", 
                  driver.acceptsURL("jdbc:snowflake://test.snowflakecomputing.com"));
        assertTrue("Should accept snowflake URL with parameters", 
                  driver.acceptsURL("jdbc:snowflake://test.snowflakecomputing.com?db=test"));
        
        // Test invalid URLs
        assertFalse("Should not accept null URL", driver.acceptsURL(null));
        assertFalse("Should not accept non-snowflake URL", 
                   driver.acceptsURL("jdbc:mysql://localhost:3306/test"));
        assertFalse("Should not accept malformed URL", 
                   driver.acceptsURL("not-a-url"));
    }
    
    @Test
    public void testDriverVersion() {
        SnowflakeDriver driver = new SnowflakeDriver();
        assertEquals("Major version should be 0", 0, driver.getMajorVersion());
        assertEquals("Minor version should be 1", 1, driver.getMinorVersion());
        assertFalse("Driver should not claim JDBC compliance in stub", driver.jdbcCompliant());
    }
    
    @Test(expected = SQLFeatureNotSupportedException.class)
    public void testGetParentLogger() throws SQLException {
        SnowflakeDriver driver = new SnowflakeDriver();
        driver.getParentLogger(); // Should throw SQLFeatureNotSupportedException
    }
    
    @Test
    public void testGetPropertyInfo() throws SQLException {
        SnowflakeDriver driver = new SnowflakeDriver();
        DriverPropertyInfo[] props = driver.getPropertyInfo(
            "jdbc:snowflake://test.snowflakecomputing.com", null);
        assertNotNull("Property info should not be null", props);
        assertEquals("Should return empty array in stub", 0, props.length);
    }
} 