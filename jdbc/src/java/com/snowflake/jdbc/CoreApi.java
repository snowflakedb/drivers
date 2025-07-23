package com.snowflake.jdbc;

import com.snowflake.jdbc.thrift_gen.DatabaseDriver;
import org.apache.thrift.protocol.TCompactProtocol;
import org.apache.thrift.transport.TTransportException;

class CoreApi {

    public enum ApiType {
        DatabaseDriverApiV1(1);
        public final int id;

        ApiType(int id) {
            this.id = id;
        }
    }

    static public DatabaseDriver.Client databaseDriverApi() {
        CoreTransport transport = new CoreTransport(ApiType.DatabaseDriverApiV1);
        try {
            transport.open();
        } catch (TTransportException e) {
            throw new RuntimeException(e);
        }
        TCompactProtocol prot = new TCompactProtocol(transport);
        DatabaseDriver.Client client = new DatabaseDriver.Client(prot);
        return client;
    }
}
