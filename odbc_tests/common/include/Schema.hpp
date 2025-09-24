#ifndef SCHEMA_HPP
#define SCHEMA_HPP

#include <chrono>
#include <random>
#include <string>

#include "Connection.hpp"

class Schema {
 public:
  Schema(Connection& conn, const std::string& schema_name) : conn(conn), schema_name(schema_name) {
    conn.execute("CREATE SCHEMA IF NOT EXISTS " + schema_name);
    conn.execute("USE SCHEMA " + schema_name);
  }

  static Schema use_random_schema(Connection& conn) {
    std::random_device rd;
    std::mt19937 gen(std::chrono::steady_clock::now().time_since_epoch().count());
    const std::string schema_name = "schema_" + std::to_string(gen());
    return Schema(conn, schema_name);
  }

  ~Schema() { conn.execute("DROP SCHEMA IF EXISTS " + schema_name); }

 private:
  Connection& conn;
  std::string schema_name;
};

#endif  // SCHEMA_HPP
