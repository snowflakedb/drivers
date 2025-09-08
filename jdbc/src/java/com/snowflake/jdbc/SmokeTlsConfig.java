package com.snowflake.jdbc;

import com.snowflake.jdbc.thrift_gen.ConnectionHandle;
import com.snowflake.jdbc.thrift_gen.DatabaseDriver;
import com.snowflake.jdbc.thrift_gen.DatabaseHandle;
import com.snowflake.jdbc.thrift_gen.TlsConfig;
import com.snowflake.jdbc.thrift_gen.CertRevocationCheckMode;

public class SmokeTlsConfig {
    public static void main(String[] args) throws Exception {
        // Ensure JNI bridge is loaded via CORE_PATH or jdbc.library.path
        DatabaseDriver.Client client = CoreApi.databaseDriverApi();
        DatabaseHandle db = client.databaseNew();
        client.databaseInit(db);
        ConnectionHandle conn = client.connectionNew();

        TlsConfig tls = new TlsConfig();
        tls.setCrl_mode(CertRevocationCheckMode.ENABLED);
        tls.setVerify_hostname(true);
        tls.setVerify_certificates(true);
        client.connectionSetTlsConfig(conn, tls);
        System.out.println("java_connectionSetTlsConfig_ok");
    }
}


