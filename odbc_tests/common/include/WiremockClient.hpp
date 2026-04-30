#ifndef WIREMOCK_CLIENT_HPP
#define WIREMOCK_CLIENT_HPP

// WiremockClient is POSIX-only: fork/exec/setsid/kill have no Windows equivalents.
// On Windows, logout.cpp skips these tests via SKIP().
#ifndef _WIN32

#include <fcntl.h>
#include <picojson.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

#include <csignal>
#include <cstring>
#include <fstream>
#include <sstream>
#include <stdexcept>
#include <string>

#include <netinet/in.h>

#include "test_setup.hpp"
#include "utils.hpp"

/// Lightweight C++ wrapper around the WireMock standalone JAR.
///
/// Mirrors the Python `WiremockClient` used by the Python logout tests:
/// starts WireMock as a subprocess, exposes admin API helpers, and
/// cleans up on destruction.
///
/// Usage:
///   WiremockClient wm;
///   wm.add_mapping_file("auth/login_success_jwt.json");
///   wm.add_mapping_file("session/logout_success.json");
///   // ... connect ODBC to wm.http_url() ...
///   CHECK(wm.get_request_count("POST", "/session") == 1);
class WiremockClient {
 public:
  WiremockClient() {
    port_ = find_free_port();
    start_process();
    wait_for_health();
  }

  ~WiremockClient() {
    if (pid_ > 0) {
      // Kill the entire process group spawned by the child to avoid orphan Java processes.
      kill(-pid_, SIGTERM);
      waitpid(pid_, nullptr, 0);
    }
  }

  // Non-copyable, non-movable (owns OS resources).
  WiremockClient(const WiremockClient&) = delete;
  WiremockClient& operator=(const WiremockClient&) = delete;

  /// Returns the HTTP base URL, e.g. "http://localhost:54321".
  std::string http_url() const { return "http://localhost:" + std::to_string(port_); }

  int port() const { return port_; }

  /// Load a mapping file from tests/wiremock/mappings/<relative_path> into WireMock.
  void add_mapping_file(const std::string& relative_path) {
    auto file_path = wiremock_mappings_dir() / relative_path;
    if (!std::filesystem::exists(file_path)) {
      throw std::runtime_error("WireMock mapping file not found: " + file_path.string());
    }
    std::string cmd = "curl -s -X POST " + admin_url("/__admin/mappings") + " -H 'Content-Type: application/json'" +
                      " --data-binary '@" + file_path.string() + "'" + " > /dev/null";
    if (std::system(cmd.c_str()) != 0) {
      throw std::runtime_error("Failed to add WireMock mapping: " + relative_path);
    }
  }

  /// Returns the number of requests WireMock has received matching the given
  /// HTTP method and URL path (exact match on path, ignoring query params).
  ///
  /// Uses WireMock's POST /__admin/requests/count admin endpoint.
  int get_request_count(const std::string& method, const std::string& url_path) const {
    std::string body = R"({"method":")" + method + R"(","urlPath":")" + url_path + R"("})";
    std::string cmd = "curl -s -X POST " + admin_url("/__admin/requests/count") +
                      " -H 'Content-Type: application/json'" + " -d '" + body + "'";
    std::string response = exec_popen(cmd);

    picojson::value json;
    std::string err = picojson::parse(json, response);
    if (!err.empty() || !json.is<picojson::object>()) {
      throw std::runtime_error("WireMock count response parse error: " + err + " | body: " + response);
    }
    const auto& obj = json.get<picojson::object>();
    auto it = obj.find("count");
    if (it == obj.end() || !it->second.is<double>()) {
      throw std::runtime_error("WireMock count response missing 'count' field: " + response);
    }
    return static_cast<int>(it->second.get<double>());
  }

 private:
  int port_;
  pid_t pid_{-1};

  std::string admin_url(const std::string& path) const { return "http://localhost:" + std::to_string(port_) + path; }

  static std::filesystem::path wiremock_mappings_dir() {
    return test_utils::repo_root() / "tests" / "wiremock" / "mappings";
  }

  static std::filesystem::path wiremock_jar_path() {
    return test_utils::repo_root() / "tests" / "wiremock" / "wiremock_standalone" / "wiremock-standalone-3.13.2.jar";
  }

  static std::filesystem::path wiremock_root_dir() { return test_utils::repo_root() / "tests" / "wiremock"; }

  /// Bind to port 0 to get an OS-assigned free port, then immediately close the socket.
  static int find_free_port() {
    int sock = ::socket(AF_INET, SOCK_STREAM, 0);
    if (sock < 0) throw std::runtime_error("Failed to create socket for port allocation");

    sockaddr_in addr{};
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = INADDR_ANY;
    addr.sin_port = 0;

    if (::bind(sock, reinterpret_cast<sockaddr*>(&addr), sizeof(addr)) < 0) {
      ::close(sock);
      throw std::runtime_error("Failed to bind to port 0");
    }

    socklen_t len = sizeof(addr);
    if (::getsockname(sock, reinterpret_cast<sockaddr*>(&addr), &len) < 0) {
      ::close(sock);
      throw std::runtime_error("Failed to get assigned port");
    }

    int port = ntohs(addr.sin_port);
    ::close(sock);
    return port;
  }

  void start_process() {
    auto jar = wiremock_jar_path();
    if (!std::filesystem::exists(jar)) {
      throw std::runtime_error("WireMock JAR not found at: " + jar.string());
    }

    auto root_dir = wiremock_root_dir();
    std::string port_str = std::to_string(port_);

    pid_ = fork();
    if (pid_ < 0) {
      throw std::runtime_error("fork() failed for WireMock process");
    }

    if (pid_ == 0) {
      // Child: create new process group so we can kill it by group.
      setsid();
      // Suppress WireMock stdout/stderr via /dev/null redirect.
      int dev_null = open("/dev/null", O_WRONLY);
      if (dev_null >= 0) {
        dup2(dev_null, STDOUT_FILENO);
        dup2(dev_null, STDERR_FILENO);
        close(dev_null);
      }
      execl("/usr/bin/java", "java", "-jar", jar.string().c_str(), "--root-dir", root_dir.string().c_str(), "--port",
            port_str.c_str(), "--proxy-pass-through", "false", "--no-request-journal", "false",
            static_cast<char*>(nullptr));
      _exit(1);
    }
    // Parent: pid_ is now the child's PID.
  }

  void wait_for_health(int timeout_secs = 15) const {
    for (int i = 0; i < timeout_secs * 5; ++i) {
      std::string cmd = "curl -s -o /dev/null -w '%{http_code}' " + admin_url("/__admin/health");
      std::string code = exec_popen(cmd);
      if (code == "200") return;
      usleep(200'000);  // 200 ms
    }
    throw std::runtime_error("WireMock did not become healthy within " + std::to_string(timeout_secs) + "s");
  }

  static std::string exec_popen(const std::string& cmd) {
    FILE* pipe = popen(cmd.c_str(), "r");
    if (!pipe) throw std::runtime_error("popen failed for: " + cmd);
    std::ostringstream ss;
    char buf[256];
    while (fgets(buf, sizeof(buf), pipe))
      ss << buf;
    int status = pclose(pipe);
    if (status == -1) {
      throw std::runtime_error("pclose failed for: " + cmd);
    }
    return ss.str();
  }
};

/// Build an ODBC connection string pointing to a running WireMock instance.
///
/// Uses the test private key from tests/test_data/invalid_rsa_key.p8
/// (same key used by sf_core Rust integration tests). WireMock accepts any
/// JWT-formatted auth request via login_success_jwt.json — no signature validation.
inline std::string get_wiremock_connection_string(const WiremockClient& wm) {
  auto key_path = test_utils::repo_root() / "tests" / "test_data" / "invalid_rsa_key.p8";
  std::ifstream key_file(key_path);
  if (!key_file) {
    throw std::runtime_error("Test private key not found at: " + key_path.string());
  }
  std::ostringstream key_ss;
  key_ss << key_file.rdbuf();
  std::string key_pem = key_ss.str();

  // configure_driver_string registers the ODBC driver with the Driver Manager
  // and prepends DRIVER={...} (Unix) or DSN=... (Windows) to the connection string.
  // Without it, unixODBC returns IM002 "Data source name not found".
  std::stringstream ss;
  configure_driver_string(ss);
  ss << "SERVER=localhost;";
  ss << "PORT=" << wm.port() << ";";
  ss << "ACCOUNT=testaccount;";
  ss << "UID=testuser;";
  ss << "PROTOCOL=http;";
  ss << "AUTHENTICATOR=SNOWFLAKE_JWT;";
  ss << "PRIV_KEY_BASE64=" << test_utils::base64_encode(key_pem) << ";";
  return ss.str();
}

#endif  // !_WIN32

#endif  // WIREMOCK_CLIENT_HPP
