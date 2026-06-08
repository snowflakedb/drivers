#ifndef SCOPED_TABLE_HPP
#define SCOPED_TABLE_HPP

#include <cstdlib>
#include <random>
#include <string>

#include "Connection.hpp"

#ifdef _WIN32
#include <process.h>
#define GET_PID() _getpid()
#else
#include <unistd.h>
#define GET_PID() getpid()
#endif

// RAII wrapper that creates a permanent table with a unique name and drops it
// on destruction.  Use this when SQL TEMPORARY tables are not suitable (e.g.
// catalog tests that check TABLE_TYPE = 'TABLE', cross-session visibility).
//
// Prefer CREATE TEMPORARY TABLE for normal data tests — it is session-scoped,
// auto-dropped on disconnect, and collision-free.
class ScopedTable {
 public:
  ScopedTable(Connection& conn, const std::string& prefix, const std::string& columns)
      : conn_(conn), name_(generate_name(prefix)) {
    conn_.execute("CREATE OR REPLACE TABLE " + name_ + " (" + columns + ")");
  }

  ~ScopedTable() {
    try {
      conn_.execute("DROP TABLE IF EXISTS " + name_);
    } catch (...) {
    }
  }

  ScopedTable(const ScopedTable&) = delete;
  ScopedTable& operator=(const ScopedTable&) = delete;

  const std::string& name() const { return name_; }

 private:
  static std::string generate_name(const std::string& prefix) {
    std::random_device rd;
    std::mt19937_64 gen(rd());
    return prefix + "_" + std::to_string(GET_PID()) + "_" + std::to_string(gen());
  }

  Connection& conn_;
  std::string name_;
};

#endif  // SCOPED_TABLE_HPP
