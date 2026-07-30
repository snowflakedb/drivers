#ifndef HANDLE_WRAPPER_HPP
#define HANDLE_WRAPPER_HPP

#include <sql.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "odbc_return_code.hpp"

class HandleWrapper {
 public:
  HandleWrapper(SQLHANDLE parent_handle, SQLSMALLINT type) : handle(SQL_NULL_HANDLE), type(type) {
    SQLRETURN ret = SQLAllocHandle(type, parent_handle, &this->handle);
    INFO("SQLAllocHandle returned " << return_code_to_string(ret));
    REQUIRE(ret == SQL_SUCCESS);
  }

  HandleWrapper(const HandleWrapper& other) = delete;
  HandleWrapper& operator=(const HandleWrapper& other) = delete;
  HandleWrapper(HandleWrapper&& other) noexcept : handle(other.handle), type(other.type) {
    other.handle = SQL_NULL_HANDLE;  // Transfer ownership
  }
  HandleWrapper& operator=(HandleWrapper&& other) noexcept {
    if (this != &other) {
      if (handle != SQL_NULL_HANDLE) {
        SQLFreeHandle(type, handle);
      }
      handle = other.handle;
      type = other.type;
      other.handle = SQL_NULL_HANDLE;
    }
    return *this;
  }

  ~HandleWrapper() {
    if (handle != SQL_NULL_HANDLE) {
      SQLFreeHandle(type, handle);
    }
  }

  SQLHANDLE getHandle() const { return handle; }
  SQLSMALLINT getType() const { return type; }

 protected:
  SQLHANDLE handle;
  SQLSMALLINT type;
};

class StatementHandleWrapper : public HandleWrapper {
 public:
  StatementHandleWrapper(SQLHANDLE parent_handle, SQLSMALLINT type) : HandleWrapper(parent_handle, type) {}
};

class ConnectionHandleWrapper : public HandleWrapper {
 public:
  ConnectionHandleWrapper(SQLHANDLE parent_handle, SQLSMALLINT type) : HandleWrapper(parent_handle, type) {}

  StatementHandleWrapper createStatementHandle() { return {this->handle, SQL_HANDLE_STMT}; }
};

class EnvironmentHandleWrapper : public HandleWrapper {
 public:
  EnvironmentHandleWrapper() : HandleWrapper(SQL_NULL_HANDLE, SQL_HANDLE_ENV) {}

  ConnectionHandleWrapper createConnectionHandle() { return {this->handle, SQL_HANDLE_DBC}; }
};

/// Takes ownership of an externally allocated, already-connected SQLHDBC.
/// Destructor calls SQLDisconnect then SQLFreeHandle so a mid-test REQUIRE
/// failure cannot leak open connections.
class ConnectedConnectionWrapper {
 public:
  explicit ConnectedConnectionWrapper(SQLHDBC h) noexcept : handle_(h) {}
  ConnectedConnectionWrapper(const ConnectedConnectionWrapper&) = delete;
  ConnectedConnectionWrapper& operator=(const ConnectedConnectionWrapper&) = delete;
  ConnectedConnectionWrapper(ConnectedConnectionWrapper&& other) noexcept : handle_(other.handle_) {
    other.handle_ = SQL_NULL_HDBC;
  }
  ~ConnectedConnectionWrapper() {
    if (handle_ != SQL_NULL_HDBC) {
      (void)SQLDisconnect(handle_);
      (void)SQLFreeHandle(SQL_HANDLE_DBC, handle_);
    }
  }
  [[nodiscard]] SQLHDBC getHandle() const noexcept { return handle_; }

 private:
  SQLHDBC handle_ = SQL_NULL_HDBC;
};

#endif  // HANDLE_WRAPPER_HPP
