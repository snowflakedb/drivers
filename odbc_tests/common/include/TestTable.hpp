#ifndef TEST_TABLE_HPP
#define TEST_TABLE_HPP

#include <string>

#include "Connection.hpp"

class TestTable {
 public:
  TestTable(Connection& conn, const std::string& name, const std::string& columns, const std::string& values)
      : conn_(conn), name_(name) {
    conn_.execute("CREATE OR REPLACE TABLE " + name_ + " (" + columns + ")");
    conn_.execute("INSERT INTO " + name_ + " VALUES " + values);
  }

  ~TestTable() { conn_.execute("DROP TABLE IF EXISTS " + name_); }

  TestTable(const TestTable&) = delete;
  TestTable& operator=(const TestTable&) = delete;

  const std::string& name() const { return name_; }

 private:
  Connection& conn_;
  std::string name_;
};

#endif  // TEST_TABLE_HPP
