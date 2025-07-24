package com.snowflake.jdbc;

import org.junit.Test;
import static org.junit.Assert.*;

import java.io.InputStream;
import java.io.InputStreamReader;
import java.sql.*;
import java.util.Properties;
import org.json.JSONObject;
import org.json.JSONTokener;

/**
 * Tests for executing queries through the Snowflake JDBC Driver
 */
public class SnowflakeQueryTest {

    private Properties loadConnectionProperties() throws Exception {
        // Load parameters.json from test resources
        String paramPath = System.getenv("PARAMETER_PATH");
        if (paramPath == null) {
            paramPath = "/parameters.json";
        }
        System.out.println("paramPath: " + paramPath);
        InputStream input = new java.io.FileInputStream(paramPath);
        if (input == null) {
            throw new RuntimeException("Could not find parameters.json in test resources");
        }

        JSONObject params = new JSONObject(new JSONTokener(new InputStreamReader(input)));
        params = params.getJSONObject("testconnection");
        
        Properties props = new Properties();
        props.setProperty("user", params.getString("SNOWFLAKE_TEST_USER"));
        props.setProperty("password", params.getString("SNOWFLAKE_TEST_PASSWORD"));
        props.setProperty("db", params.getString("SNOWFLAKE_TEST_DATABASE"));
        props.setProperty("schema", params.getString("SNOWFLAKE_TEST_SCHEMA"));
        props.setProperty("warehouse", params.getString("SNOWFLAKE_TEST_WAREHOUSE"));
        props.setProperty("account", params.getString("SNOWFLAKE_TEST_ACCOUNT"));
        
        return props;
    }

    @Test
    public void testSimpleSelect() throws Exception {
        // Load connection properties
        Properties props = loadConnectionProperties();
        String url = props.getProperty("url", "jdbc:snowflake://" + props.getProperty("account") + ".snowflakecomputing.com");
        
        // Create connection
        SnowflakeDriver.empty();
        Connection conn = DriverManager.getConnection(url, props);
        assertNotNull("Connection should not be null", conn);
        
        try {
            // Create and execute statement
            Statement stmt = conn.createStatement();
            ResultSet rs = stmt.executeQuery("SELECT 1");
            
            // Verify result
            assertNotNull("ResultSet should not be null", rs);
            assertTrue("ResultSet should have one row", rs.next());
            assertEquals("Result should be 1", 1, rs.getInt(1));

            // Clean up
            rs.close();
            stmt.close();
        } finally {
            conn.close();
        }
    }
}

