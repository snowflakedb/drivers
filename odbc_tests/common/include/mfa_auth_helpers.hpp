#ifndef MFA_AUTH_HELPERS_HPP
#define MFA_AUTH_HELPERS_HPP

#include <picojson.h>

#include <cctype>
#include <cerrno>
#include <chrono>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <set>
#include <sstream>
#include <stdexcept>
#include <string>
#include <system_error>
#include <thread>
#include <utility>
#include <vector>

#ifndef _WIN32
#include <fcntl.h>
#include <sys/file.h>
#include <sys/wait.h>
#include <unistd.h>
#else
#include <process.h>
#endif

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
// Matches totpGenerator.js MIN_VALIDITY_SECONDS. Image :4 does not wait
// internally; callers must skip a soon-to-expire current window themselves.
constexpr int MIN_TOTP_VALIDITY_SECONDS = 8;

inline std::set<std::string>& used_totp_codes() {
  static std::set<std::string> codes;
  return codes;
}

inline std::string mfa_build_tag() {
  const char* tag = std::getenv("BUILD_TAG");
  if (tag != nullptr && tag[0] != '\0') {
    return tag;
  }
  return "local";
}

inline std::string mfa_state_dir() {
  // Prefer WORKSPACE_ROOT. run_auth_browser.sh bind-mounts the repo at
  // /mnt/host and sets WORKSPACE_ROOT=/mnt/host (WORKSPACE is remapped to
  // the same). A host Jenkins WORKSPACE path is not usable inside the
  // container. BUILD_TAG namespaces the dir so a leftover exhausted flag
  // from a prior job on a reused workspace cannot skip this run.
  // Same-agent only (Catch2, universal→reference) — not across language agents.
  const char* workspace_root = std::getenv("WORKSPACE_ROOT");
  if (workspace_root != nullptr && workspace_root[0] != '\0') {
    return std::string(workspace_root) + "/.ud-mfa-totp-state/" + mfa_build_tag();
  }
  const char* workspace = std::getenv("WORKSPACE");
  if (workspace != nullptr && workspace[0] != '\0') {
    return std::string(workspace) + "/.ud-mfa-totp-state/" + mfa_build_tag();
  }
#ifndef _WIN32
  const char* tag = std::getenv("BUILD_TAG");
  if (tag != nullptr && tag[0] != '\0') {
    return std::string("/tmp/ud-mfa-") + tag;
  }
  // Local ctest: share across Catch2 processes from the same parent, not across runs.
  return std::string("/tmp/ud-mfa-") + std::to_string(::getppid());
#else
  const char* temp = std::getenv("TEMP");
  if (temp != nullptr && temp[0] != '\0') {
    return temp;
  }
  return std::string(".\\ud-mfa-") + std::to_string(_getpid());
#endif
}

inline void ensure_mfa_state_dir() {
#ifndef _WIN32
  // Parent `.ud-mfa-totp-state` is gitignored and absent on a fresh language-agent workspace.
  std::error_code ec;
  std::filesystem::create_directories(mfa_state_dir(), ec);
#endif
}

inline std::string used_codes_path() { return mfa_state_dir() + "/ud-mfa-used-totp-codes"; }

inline std::string exhausted_flag_path() { return mfa_state_dir() + "/ud-mfa-connect-exhausted"; }

#ifndef _WIN32
struct FileLock {
  int fd = -1;
  explicit FileLock(const std::string& path) {
    fd = ::open(path.c_str(), O_RDWR | O_CREAT, 0600);
    if (fd < 0) {
      throw std::runtime_error("failed to open MFA TOTP state file: " + path + ": " + std::strerror(errno));
    }
    if (::flock(fd, LOCK_EX) != 0) {
      const int lock_err = errno;
      ::close(fd);
      fd = -1;
      throw std::runtime_error("failed to lock MFA TOTP state file: " + path + ": " + std::strerror(lock_err));
    }
  }
  ~FileLock() {
    if (fd >= 0) {
      ::flock(fd, LOCK_UN);
      ::close(fd);
    }
  }
  FileLock(const FileLock&) = delete;
  FileLock& operator=(const FileLock&) = delete;
};
#endif

// Circuit breaker: 394512 → mark + skip; budget exhaust after >=1 submit →
// mark + fail; zero submits → fail without marking.
inline bool shared_mfa_exhausted() {
#ifndef _WIN32
  return ::access(exhausted_flag_path().c_str(), F_OK) == 0;
#else
  return false;
#endif
}

inline void mark_shared_mfa_exhausted() {
#ifndef _WIN32
  ensure_mfa_state_dir();
  std::ofstream out(exhausted_flag_path());
  out << "1\n";
#endif
}

inline bool claim_totp_code(const std::string& code) {
#ifndef _WIN32
  ensure_mfa_state_dir();
  FileLock lock(used_codes_path());
  std::set<std::string> codes;
  std::ifstream in(used_codes_path());
  std::string line;
  while (std::getline(in, line)) {
    while (!line.empty() && (line.back() == '\r' || line.back() == ' ')) {
      line.pop_back();
    }
    if (!line.empty()) {
      codes.insert(line);
    }
  }
  in.close();
  if (codes.count(code) != 0) {
    return false;
  }
  std::ofstream out(used_codes_path(), std::ios::app);
  if (!out) {
    return false;
  }
  out << code << '\n';
  return static_cast<bool>(out);
#else
  auto& codes = used_totp_codes();
  if (codes.count(code) != 0) {
    return false;
  }
  codes.insert(code);
  return true;
#endif
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

inline double totp_seconds_remaining() {
  using namespace std::chrono;
  const double elapsed = duration<double>(system_clock::now().time_since_epoch()).count();
  const double remaining = TOTP_STEP_SECONDS - std::fmod(elapsed, TOTP_STEP_SECONDS);
  return remaining > 0 ? remaining : 0.0;
}

inline long totp_window_id() {
  using namespace std::chrono;
  return static_cast<long>(duration<double>(system_clock::now().time_since_epoch()).count()) / TOTP_STEP_SECONDS;
}

inline void wait_if_near_totp_boundary() {
  const double remaining = totp_seconds_remaining();
  if (remaining < MIN_TOTP_VALIDITY_SECONDS) {
    std::this_thread::sleep_for(std::chrono::duration<double>(remaining + 1.0));
  }
}

inline bool is_six_digit_token(const std::string& token) {
  if (token.size() != 6) {
    return false;
  }
  for (char ch : token) {
    if (!std::isdigit(static_cast<unsigned char>(ch))) {
      return false;
    }
  }
  return true;
}

#ifndef _WIN32
// macOS does not ship GNU timeout; never invoke an unqualified `timeout`.
// On Linux resolve from PATH, then the FHS locations.
inline std::string timeout_prefix() {
#ifdef __APPLE__
  return {};
#else
  if (const char* path = std::getenv("PATH")) {
    std::istringstream ss(path);
    std::string dir;
    while (std::getline(ss, dir, ':')) {
      if (dir.empty()) {
        continue;
      }
      const std::string cand = dir + "/timeout";
      if (::access(cand.c_str(), X_OK) == 0) {
        return "'" + cand + "' 40 ";
      }
    }
  }
  if (::access("/usr/bin/timeout", X_OK) == 0) {
    return "/usr/bin/timeout 40 ";
  }
  if (::access("/bin/timeout", X_OK) == 0) {
    return "/bin/timeout 40 ";
  }
  return {};
#endif
}
#endif

inline std::string exec_totp_generator(const std::string& seed) {
  // Seed goes through the env var (same as Win32 / Java). Never interpolate
  // it into a shell string — a `'` in the seed would break quoting.
  EnvOverride seed_guard("SNOWFLAKE_AUTH_MFA_SEED", seed);
#ifdef _WIN32
  return platform::exec_command(std::string("node ") + TOTP_GENERATOR_SCRIPT);
#else
  const std::string cmd = timeout_prefix() + "node " + TOTP_GENERATOR_SCRIPT;
  FILE* pipe = popen(cmd.c_str(), "r");
  if (!pipe) {
    FAIL("popen failed for totpGenerator.js");
  }
  std::ostringstream ss;
  char buf[256];
  while (fgets(buf, sizeof(buf), pipe)) {
    ss << buf;
  }
  const int status = pclose(pipe);
  if (status == -1) {
    FAIL("pclose failed for totpGenerator.js");
  }
  if (!WIFEXITED(status)) {
    FAIL("totpGenerator.js did not exit normally");
  }
  const int rc = WEXITSTATUS(status);
  if (rc == 124) {
    FAIL("totpGenerator.js timed out after 40s");
  }
  if (rc != 0) {
    FAIL("totpGenerator.js failed (rc=" << rc << ")");
  }
  return ss.str();
#endif
}

inline std::string get_current_totp_code(const std::string& seed) {
  wait_if_near_totp_boundary();
  std::istringstream iss(exec_totp_generator(seed));
  std::vector<std::string> codes;
  std::string token;
  while (iss >> token) {
    if (is_six_digit_token(token)) {
      codes.push_back(token);
    }
  }
  if (codes.empty()) {
    FAIL("totpGenerator.js produced no TOTP code");
  }

  // Image :4 emits past/current/future (or current/future near a boundary). Newer helpers emit
  // only current. Second-to-last is the current token on both; wait_if_near_totp_boundary
  // avoids submitting one with <8s left.
  return codes.size() == 1 ? codes.front() : codes[codes.size() - 2];
}

inline std::string fresh_totp_code(const std::string& seed) {
  const std::string code = get_current_totp_code(seed);
  return claim_totp_code(code) ? code : "";
}

inline void sleep_to_next_totp_window() {
  double wait = totp_seconds_remaining();
  if (wait > 0) {
    wait += 1.0;
    std::this_thread::sleep_for(std::chrono::duration<double>(wait));
  }
}

inline void sleep_if_still_in_window(long window_id) {
  if (totp_window_id() == window_id) {
    sleep_to_next_totp_window();
  }
}

inline std::string acquire_totp_passcode(const std::string& seed, int max_windows = 3) {
  int advances = 0;
  while (advances < max_windows) {
    const std::string passcode = fresh_totp_code(seed);
    if (!passcode.empty()) {
      return passcode;
    }
    sleep_to_next_totp_window();
    ++advances;
  }
  FAIL("No unused TOTP passcodes available after " << max_windows << " windows");
  return "";
}

inline bool message_has_ci_insensitive(const std::string& msg, const char* needle) {
  std::string lower = msg;
  for (auto& ch : lower) {
    ch = static_cast<char>(std::tolower(static_cast<unsigned char>(ch)));
  }
  return lower.find(needle) != std::string::npos;
}

inline bool is_totp_retryable_error(const std::vector<DiagRec>& records) {
  for (const auto& record : records) {
    const std::string& msg = record.messageText;
    if (msg.find("TOTP Invalid") != std::string::npos || message_has_ci_insensitive(msg, "invalid passcode")) {
      return true;
    }
  }
  return false;
}

inline bool is_mfa_lockout_error(const std::vector<DiagRec>& records) {
  for (const auto& record : records) {
    const std::string& msg = record.messageText;
    if (msg.find("394512") != std::string::npos || message_has_ci_insensitive(msg, "too many failed mfa")) {
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
    // BD#84: the legacy ODBC driver only recognizes the value "on" (case-insensitive)
    // for PASSCODEINPASSWORD, whereas the new driver accepts "on"/"true"/"1". Use "on"
    // so the shared e2e flow runs against either driver.
    ss << "PASSCODEINPASSWORD=on;";
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
  if (shared_mfa_exhausted()) {
    SKIP("Shared MFA account already exhausted TOTP retries in this run");
  }

  std::string last_error;
  int submits = 0;
  int advances = 0;
  while (submits < max_windows) {
    const std::string passcode = fresh_totp_code(totp_seed);
    if (passcode.empty()) {
      if (advances >= max_windows) {
        break;
      }
      sleep_to_next_totp_window();
      ++advances;
      continue;
    }

    const long window_id = totp_window_id();
    ++submits;
    const std::string password = passcode_in_password ? base_password + passcode : base_password;
    const std::string* passcode_ptr = passcode_in_password ? nullptr : &passcode;
    const std::string connection_string =
        build_mfa_connection_string(params, password, passcode_ptr, passcode_in_password, extra);

    auto dbc = env.createConnectionHandle();
    SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, (SQLCHAR*)connection_string.c_str(), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    if (ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
      return dbc;
    }

    auto records = get_diag_rec(dbc);
    if (is_mfa_lockout_error(records)) {
      mark_shared_mfa_exhausted();
      SKIP("Shared MFA account locked (394512); skipping this and later MFA tests");
    }
    if (is_totp_retryable_error(records)) {
      last_error = records.empty() ? "unknown TOTP error" : records[0].messageText;
    } else {
      REQUIRE_ODBC(ret, dbc);
    }

    if (submits < max_windows) {
      sleep_if_still_in_window(window_id);
    }
  }

  if (submits == 0) {
    FAIL("No unused TOTP passcodes after " << max_windows << " windows");
  }
  mark_shared_mfa_exhausted();
  FAIL("Failed to connect after " << submits << " TOTP submits. Last error: " << last_error);
  return env.createConnectionHandle();
}

}  // namespace mfa_auth

#endif  // MFA_AUTH_HELPERS_HPP
