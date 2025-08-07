#ifndef HANDLE_WRAPPER_HPP
#define HANDLE_WRAPPER_HPP

#include <sql.h>
#include <sqltypes.h>

#include "macros.hpp"

class HandleWrapper {
 public:
  HandleWrapper(SQLHANDLE parent_handle, SQLSMALLINT type) : handle(SQL_NULL_HANDLE), type(type) {
    SQLRETURN ret = SQLAllocHandle(type, parent_handle, &this->handle);
    CHECK_ODBC_ERROR(ret, handle, type);
  }

  HandleWrapper(const HandleWrapper& other) = delete;
  HandleWrapper& operator=(const HandleWrapper& other) = delete;
  HandleWrapper(HandleWrapper&& other) noexcept : handle(other.handle), type(other.type) {
    other.handle = SQL_NULL_HANDLE;  // Transfer ownership
  }

  ~HandleWrapper() { SQLFreeHandle(type, handle); }

  SQLHANDLE getHandle() const { return handle; }
  SQLSMALLINT getType() const { return type; }

 protected:
  SQLHANDLE handle;
  SQLSMALLINT type;
};

class StatementHandleWrapper : public HandleWrapper {
 public:
  StatementHandleWrapper(SQLHANDLE parent_handle, SQLSMALLINT type)
      : HandleWrapper(parent_handle, type) {}
};

class ConnectionHandleWrapper : public HandleWrapper {
 public:
  ConnectionHandleWrapper(SQLHANDLE parent_handle, SQLSMALLINT type)
      : HandleWrapper(parent_handle, type) {}

  StatementHandleWrapper createStatementHandle() { return {this->handle, SQL_HANDLE_STMT}; }
};

class EnvironmentHandleWrapper : public HandleWrapper {
 public:
  EnvironmentHandleWrapper() : HandleWrapper(SQL_NULL_HANDLE, SQL_HANDLE_ENV) {}

  ConnectionHandleWrapper createConnectionHandle() { return {this->handle, SQL_HANDLE_DBC}; }
};

#endif  // HANDLE_WRAPPER_HPP
