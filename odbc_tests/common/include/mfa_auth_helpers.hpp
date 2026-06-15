#ifndef MFA_AUTH_HELPERS_HPP
#define MFA_AUTH_HELPERS_HPP

#include <picojson.h>

#include <cctype>
#include <chrono>
#include <cmath>
#include <cstdlib>
#include <set>
#include <sstream>
#include <string>
#include <thread>
#include <utility>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "EnvOverride.hpp"
#include "HandleWrapper.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"
#include "platform.hpp"
#include "test_setup.hpp"

// TOTP passcode helpers for USERNAME_PASSWORD_MFA E2E tests.
//
// Requires the snowdrivers-test-external-browser-universal-driver Docker container
// (/externalbrowser/totpGenerator.js generates TOTP passcodes for the MFA test user).
// Mirrors python/tests/e2e/authentication/auth_helpers.py.
//
// Callers must invoke ensure_driver_installed() before allocating an ODBC environment
// handle — unixODBC reads ODBCSYSINI at SQLAllocHandle(SQL_HANDLE_ENV) time.

namespace mfa_auth {

constexpr const char* TOTP_GENERATOR_SCRIPT = "/externalbrowser/totpGenerator.js";
constexpr int TOTP_STEP_SECONDS = 30;

inline std::set<std::string>& used_totp_codes() {
  static std::set<std::string> codes;
  return codes;
}

inline picojson::object get_mfa_test_parameters() {
  auto params = get_test_parameters("testconnection");
  const char* mfa_keys[] = {"SNOWFLAKE_TEST_MFA_USER", "SNOWFLAKE_TEST_MFA_PASSWORD", "SNOWFLAKE_TEST_MFA_SEED"};
  for (const char* key : mfa_keys) {
    const char* val = std::getenv(key);
    if (val != nullptr && val[0] != '\0') {
      params[key] = picojson::value(val);
    }
  }
  return params;
}

struct MfaCredentials {
  std::string password;
  std::string totp_seed;
};

inline MfaCredentials load_mfa_credentials() {
  auto params = get_mfa_test_parameters();
  get_param_required<std::string>(params, "SNOWFLAKE_TEST_MFA_USER");
  MfaCredentials creds{
      get_param_required<std::string>(params, "SNOWFLAKE_TEST_MFA_PASSWORD"),
      get_param_required<std::string>(params, "SNOWFLAKE_TEST_MFA_SEED"),
  };
  return creds;
}

inline std::vector<std::string> get_totp_codes(const std::string& seed) {
  // totpGenerator.js reads SNOWFLAKE_AUTH_MFA_SEED; restore the prior value afterwards.
  EnvOverride seed_guard("SNOWFLAKE_AUTH_MFA_SEED", seed);
  std::string output = platform::exec_command(std::string("node ") + TOTP_GENERATOR_SCRIPT);
  std::istringstream iss(output);
  std::vector<std::string> codes;
  std::string code;
  while (iss >> code) {
    codes.push_back(code);
  }
  if (codes.empty()) {
    FAIL("totpGenerator.js produced no TOTP codes");
  }
  return codes;
}

inline std::vector<std::string> fresh_totp_codes(const std::string& seed) {
  std::vector<std::string> fresh;
  for (const auto& code : get_totp_codes(seed)) {
    if (!used_totp_codes().count(code)) {
      fresh.push_back(code);
    }
  }
  return fresh;
}

inline void sleep_to_next_totp_window() {
  using namespace std::chrono;
  const auto now = system_clock::now().time_since_epoch();
  const double elapsed = duration<double>(now).count();
  double wait = TOTP_STEP_SECONDS - std::fmod(elapsed, TOTP_STEP_SECONDS);
  if (wait > 0) {
    wait += 1.0;
    std::this_thread::sleep_for(duration<double>(wait));
  }
}

inline std::string acquire_totp_passcode(const std::string& seed, int max_windows = 3) {
  for (int window = 0; window < max_windows; ++window) {
    auto fresh = fresh_totp_codes(seed);
    if (!fresh.empty()) {
      used_totp_codes().insert(fresh.front());
      return fresh.front();
    }
    if (window < max_windows - 1) {
      sleep_to_next_totp_window();
    }
  }
  FAIL("No unused TOTP passcodes available after " << max_windows << " windows");
  return "";
}

inline bool is_totp_retryable_error(const std::vector<DiagRec>& records) {
  for (const auto& record : records) {
    const std::string& msg = record.messageText;
    if (msg.find("TOTP Invalid") != std::string::npos) {
      return true;
    }
    std::string lower = msg;
    for (auto& ch : lower) {
      ch = static_cast<char>(std::tolower(static_cast<unsigned char>(ch)));
    }
    if (lower.find("invalid passcode") != std::string::npos) {
      return true;
    }
  }
  return false;
}

inline std::string build_mfa_connection_string(const picojson::object& params, const std::string& password,
                                               const std::string* passcode, bool passcode_in_password,
                                               const std::vector<std::pair<std::string, std::string>>& extra = {}) {
  std::stringstream ss;
  read_default_params(ss, params, {"UID", "AUTHENTICATOR", "PWD", "ROLE"});
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_MFA_USER", "UID");
  ss << "PWD=" << password << ";";
  ss << "AUTHENTICATOR=USERNAME_PASSWORD_MFA;";
  ss << "ROLE=PUBLIC;";
  if (passcode_in_password) {
    ss << "PASSCODEINPASSWORD=true;";
  } else if (passcode != nullptr) {
    ss << "PASSCODE=" << *passcode << ";";
  }
  for (const auto& [key, value] : extra) {
    ss << key << "=" << value << ";";
  }
  return ss.str();
}

inline ConnectionHandleWrapper connect_with_totp_retry(
    EnvironmentHandleWrapper& env, const std::string& totp_seed, const std::string& base_password,
    bool passcode_in_password, const picojson::object& params,
    const std::vector<std::pair<std::string, std::string>>& extra = {}, int max_windows = 3) {
  std::string last_error;
  for (int window = 0; window < max_windows; ++window) {
    auto codes = fresh_totp_codes(totp_seed);
    if (codes.empty()) {
      if (window < max_windows - 1) {
        sleep_to_next_totp_window();
        continue;
      }
      break;
    }

    for (const auto& passcode : codes) {
      used_totp_codes().insert(passcode);
      const std::string password = passcode_in_password ? base_password + passcode : base_password;
      const std::string* passcode_ptr = passcode_in_password ? nullptr : &passcode;
      const std::string connection_string =
          build_mfa_connection_string(params, password, passcode_ptr, passcode_in_password, extra);

      auto dbc = env.createConnectionHandle();
      SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, (SQLCHAR*)connection_string.c_str(), SQL_NTS, nullptr,
                                       0, nullptr, SQL_DRIVER_NOPROMPT);
      if (ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
        return dbc;
      }

      auto records = get_diag_rec(dbc);
      if (is_totp_retryable_error(records)) {
        last_error = records.empty() ? "unknown TOTP error" : records[0].messageText;
        continue;
      }
      REQUIRE_ODBC(ret, dbc);
    }

    if (window < max_windows - 1) {
      sleep_to_next_totp_window();
    }
  }

  FAIL("Failed to connect after " << max_windows << " TOTP windows. Last error: " << last_error);
  return env.createConnectionHandle();
}

}  // namespace mfa_auth

#endif  // MFA_AUTH_HELPERS_HPP
