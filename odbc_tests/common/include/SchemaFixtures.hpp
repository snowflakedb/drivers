#ifndef SCHEMA_FIXTURES_HPP
#define SCHEMA_FIXTURES_HPP

#include "ODBCFixtures.hpp"
#include "Schema.hpp"

// Connection RAII fixtures (SQLDriverConnect via connection string)
// Prefixed with "Conn" to distinguish from raw-handle fixtures in ODBCFixtures.hpp.

struct ConnFixture {
  Connection conn;
};

struct ConnSchemaFixture {
  Connection conn;
  ConnSchemaFixture() { Schema::use_temp_session_schema(conn); }
};

// Raw-handle fixture with session schema (extends existing StmtDefaultDSNFixture)

struct StmtSessionSchemaFixture : StmtDefaultDSNFixture {
  StmtSessionSchemaFixture() { Schema::use_temp_session_schema(dbc_handle()); }
};

#endif  // SCHEMA_FIXTURES_HPP
