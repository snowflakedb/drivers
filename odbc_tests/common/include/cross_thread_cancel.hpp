#pragma once

#include <sql.h>
#include <sqlext.h>

#include <atomic>
#include <chrono>
#include <condition_variable>
#include <mutex>
#include <thread>

#include "odbc_cast.hpp"

namespace odbc_test {

struct JoinGuard {
  std::thread& t;
  ~JoinGuard() {
    if (t.joinable()) t.join();
  }
  JoinGuard(const JoinGuard&) = delete;
  JoinGuard& operator=(const JoinGuard&) = delete;
};

struct CrossThreadCancel {
  std::atomic<SQLRETURN> exec_result{SQL_NO_DATA};
  SQLRETURN cancel_result = SQL_ERROR;

  void run(SQLHSTMT stmt, const char* query, std::chrono::seconds pre_cancel_delay) {
    std::mutex mtx;
    std::condition_variable cv;
    bool executing = false;

    std::thread executor([&]() {
      {
        std::lock_guard lk(mtx);
        executing = true;
      }
      cv.notify_one();
      exec_result.store(SQLExecDirect(stmt, sqlchar(query), SQL_NTS));
    });
    JoinGuard guard{executor};

    {
      std::unique_lock lk(mtx);
      cv.wait(lk, [&] { return executing; });
    }

    if (pre_cancel_delay.count() > 0) {
      std::this_thread::sleep_for(pre_cancel_delay);
    }

    cancel_result = SQLCancel(stmt);
  }
};

}  // namespace odbc_test
