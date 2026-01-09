package com.snowflake.jdbc.e2e.query;

import com.snowflake.jdbc.SnowflakeDriver;
import org.junit.Test;
import static org.junit.Assert.*;

import java.io.InputStream;
import java.io.InputStreamReader;
import java.sql.*;
import java.util.Properties;
import org.json.JSONObject;
import org.json.JSONTokener;

/**
 * Tests for empty result handling
 */
public class EmptyResultTest {

    private Properties loadConnectionProperties() throws Exception {
        String paramPath = System.getenv("PARAMETER_PATH");
        if (paramPath == null) {
            paramPath = "/parameters.json";
        }
        InputStream input = new java.io.FileInputStream(paramPath);
        if (input == null) {
            throw new RuntimeException("Could not find parameters.json");
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
        
        if (params.has("SNOWFLAKE_TEST_PORT")) {
            props.setProperty("port", String.valueOf(params.getInt("SNOWFLAKE_TEST_PORT")));
        }
        if (params.has("SNOWFLAKE_TEST_ROLE")) {
            props.setProperty("role", params.getString("SNOWFLAKE_TEST_ROLE"));
        }
        
        return props;
    }

    /**
     * Test: should return empty result when query produces no rows
     */
    @Test
    public void shouldReturnEmptyResultWhenQueryProducesNoRows() throws Exception {
        // Given Snowflake client is logged in
        Properties props = loadConnectionProperties();
        String url = "jdbc:snowflake://" + props.getProperty("account") + ".snowflakecomputing.com";
        if (props.getProperty("port") != null) {
            url += ":" + props.getProperty("port");
        }
        
        SnowflakeDriver.empty();
        Connection conn = DriverManager.getConnection(url, props);

        try {
            // When Query "SELECT 1 WHERE FALSE" is executed
            Statement stmt = conn.createStatement();
            ResultSet rs = stmt.executeQuery("SELECT 1 WHERE FALSE");
            
            // Then empty result set is returned
            assertNotNull("ResultSet should not be null", rs);
            assertFalse("ResultSet should have no rows", rs.next());

            rs.close();
            stmt.close();
        } finally {
            conn.close();
        }
    }
}

