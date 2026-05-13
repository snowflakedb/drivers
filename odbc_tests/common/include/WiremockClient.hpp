#ifndef WIREMOCK_CLIENT_HPP
#define WIREMOCK_CLIENT_HPP

#include <picojson.h>

#include <cstdlib>
#include <fstream>
#include <sstream>
#include <stdexcept>
#include <string>
#include <thread>

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <winsock2.h>
#include <ws2tcpip.h>
#pragma comment(lib, "ws2_32.lib")
#else
#include <fcntl.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

#include <csignal>

#include <netinet/in.h>
#endif

#include "utils.hpp"

/// Cross-platform wrapper around the WireMock standalone JAR.
///
/// Starts WireMock as a subprocess, exposes admin API helpers, and
/// cleans up on destruction.
class WiremockClient {
 public:
  WiremockClient() {
    port_ = find_free_port();
    start_process();
    wait_for_health();
  }

  ~WiremockClient() { stop_process(); }

  WiremockClient(const WiremockClient&) = delete;
  WiremockClient& operator=(const WiremockClient&) = delete;

  std::string http_url() const { return "http://localhost:" + std::to_string(port_); }

  int port() const { return port_; }

  void add_mapping_file(const std::string& relative_path) {
    auto file_path = wiremock_mappings_dir() / relative_path;
    if (!std::filesystem::exists(file_path)) {
      throw std::runtime_error("WireMock mapping file not found: " + file_path.string());
    }
    std::string cmd = "curl -s -X POST " + admin_url("/__admin/mappings") +
                      " -H \"Content-Type: application/json\""
                      " --data-binary \"@" +
                      file_path.string() + "\"" + null_redirect();
    if (std::system(cmd.c_str()) != 0) {
      throw std::runtime_error("Failed to add WireMock mapping: " + relative_path);
    }
  }

  int get_request_count(const std::string& method, const std::string& url_path) const {
    std::string body = R"({"method":")" + method + R"(","urlPath":")" + url_path + R"("})";

    auto tmp = std::filesystem::temp_directory_path() / "wm_request_count.json";
    {
      std::ofstream f(tmp);
      f << body;
    }
    std::string cmd = "curl -s -X POST " + admin_url("/__admin/requests/count") +
                      " -H \"Content-Type: application/json\""
                      " --data-binary \"@" +
                      tmp.string() + "\"";
    std::string response = exec_popen(cmd);
    std::filesystem::remove(tmp);

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

#ifdef _WIN32
  HANDLE process_handle_{INVALID_HANDLE_VALUE};
  HANDLE job_handle_{INVALID_HANDLE_VALUE};
#else
  pid_t pid_{-1};
#endif

  std::string admin_url(const std::string& path) const { return "http://localhost:" + std::to_string(port_) + path; }

  static std::string null_redirect() {
#ifdef _WIN32
    return " > NUL 2>&1";
#else
    return " > /dev/null 2>&1";
#endif
  }

  static std::filesystem::path wiremock_mappings_dir() {
    return test_utils::repo_root() / "tests" / "wiremock" / "mappings";
  }

  static std::filesystem::path wiremock_jar_path() {
    return test_utils::repo_root() / "tests" / "wiremock" / "wiremock_standalone" / "wiremock-standalone-3.13.2.jar";
  }

  static std::filesystem::path wiremock_root_dir() { return test_utils::repo_root() / "tests" / "wiremock"; }

  static int find_free_port() {
#ifdef _WIN32
    WSADATA wsa_data;
    WSAStartup(MAKEWORD(2, 2), &wsa_data);
    SOCKET sock = ::socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (sock == INVALID_SOCKET) throw std::runtime_error("Failed to create socket for port allocation");

    sockaddr_in addr{};
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = INADDR_ANY;
    addr.sin_port = 0;

    if (::bind(sock, reinterpret_cast<sockaddr*>(&addr), sizeof(addr)) == SOCKET_ERROR) {
      closesocket(sock);
      throw std::runtime_error("Failed to bind to port 0");
    }

    int len = sizeof(addr);
    if (::getsockname(sock, reinterpret_cast<sockaddr*>(&addr), &len) == SOCKET_ERROR) {
      closesocket(sock);
      throw std::runtime_error("Failed to get assigned port");
    }

    int port = ntohs(addr.sin_port);
    closesocket(sock);
    return port;
#else
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
#endif
  }

  void start_process() {
    auto jar = wiremock_jar_path();
    if (!std::filesystem::exists(jar)) {
      throw std::runtime_error("WireMock JAR not found at: " + jar.string());
    }

    auto root_dir = wiremock_root_dir();
    std::string port_str = std::to_string(port_);

#ifdef _WIN32
    // Create a Job Object so all child processes are killed when the job is closed.
    job_handle_ = CreateJobObject(nullptr, nullptr);
    if (job_handle_ != nullptr) {
      JOBOBJECT_EXTENDED_LIMIT_INFORMATION job_info{};
      job_info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
      SetInformationJobObject(job_handle_, JobObjectExtendedLimitInformation, &job_info, sizeof(job_info));
    }

    std::string cmd_line = "java -jar \"" + jar.string() + "\" --root-dir \"" + root_dir.string() + "\" --port " +
                           port_str + " --proxy-pass-through false";

    STARTUPINFOA si{};
    si.cb = sizeof(si);
    si.dwFlags = STARTF_USESTDHANDLES;
    si.hStdInput = INVALID_HANDLE_VALUE;
    si.hStdOutput = INVALID_HANDLE_VALUE;
    si.hStdError = INVALID_HANDLE_VALUE;

    PROCESS_INFORMATION pi{};
    BOOL ok = CreateProcessA(nullptr, cmd_line.data(), nullptr, nullptr, FALSE,
                             CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP, nullptr, nullptr, &si, &pi);
    if (!ok) {
      throw std::runtime_error("CreateProcess failed for WireMock (error " + std::to_string(GetLastError()) + ")");
    }

    process_handle_ = pi.hProcess;
    CloseHandle(pi.hThread);

    if (job_handle_ != nullptr) {
      AssignProcessToJobObject(job_handle_, process_handle_);
    }
#else
    pid_ = fork();
    if (pid_ < 0) {
      throw std::runtime_error("fork() failed for WireMock process");
    }

    if (pid_ == 0) {
      setsid();
      int dev_null = open("/dev/null", O_WRONLY);
      if (dev_null >= 0) {
        dup2(dev_null, STDOUT_FILENO);
        dup2(dev_null, STDERR_FILENO);
        close(dev_null);
      }
      execlp("java", "java", "-jar", jar.string().c_str(), "--root-dir", root_dir.string().c_str(), "--port",
             port_str.c_str(), "--proxy-pass-through", "false", static_cast<char*>(nullptr));
      _exit(1);
    }
#endif
  }

  void stop_process() {
#ifdef _WIN32
    if (process_handle_ != INVALID_HANDLE_VALUE) {
      TerminateProcess(process_handle_, 0);
      WaitForSingleObject(process_handle_, 5000);
      CloseHandle(process_handle_);
      process_handle_ = INVALID_HANDLE_VALUE;
    }
    if (job_handle_ != INVALID_HANDLE_VALUE) {
      CloseHandle(job_handle_);
      job_handle_ = INVALID_HANDLE_VALUE;
    }
#else
    if (pid_ > 0) {
      kill(-pid_, SIGTERM);
      waitpid(pid_, nullptr, 0);
      pid_ = -1;
    }
#endif
  }

  void wait_for_health(int timeout_secs = 15) const {
    for (int i = 0; i < timeout_secs * 5; ++i) {
      std::string cmd = "curl -s -o " +
#ifdef _WIN32
                        std::string("NUL") +
#else
                        std::string("/dev/null") +
#endif
                        " -w \"%{http_code}\" " + admin_url("/__admin/health");
      std::string code = exec_popen(cmd);
      if (code == "200") return;
      std::this_thread::sleep_for(std::chrono::milliseconds(200));
    }
    throw std::runtime_error("WireMock did not become healthy within " + std::to_string(timeout_secs) + "s");
  }

  static std::string exec_popen(const std::string& cmd) {
#ifdef _WIN32
    FILE* pipe = _popen(cmd.c_str(), "r");
#else
    FILE* pipe = popen(cmd.c_str(), "r");
#endif
    if (!pipe) throw std::runtime_error("popen failed for: " + cmd);
    std::ostringstream ss;
    char buf[256];
    while (fgets(buf, sizeof(buf), pipe))
      ss << buf;
#ifdef _WIN32
    int status = _pclose(pipe);
#else
    int status = pclose(pipe);
#endif
    if (status == -1) {
      throw std::runtime_error("pclose failed for: " + cmd);
    }
    return ss.str();
  }
};

/// Build an ODBC connection string pointing to a running WireMock instance.
inline std::string get_wiremock_connection_string(const WiremockClient& wm) {
  auto key_path = test_utils::repo_root() / "tests" / "test_data" / "invalid_rsa_key.p8";
  std::ifstream key_file(key_path);
  if (!key_file) {
    throw std::runtime_error("Test private key not found at: " + key_path.string());
  }
  std::ostringstream key_ss;
  key_ss << key_file.rdbuf();
  std::string key_pem = key_ss.str();

  const char* driver_path_env = std::getenv("DRIVER_PATH");
  if (driver_path_env == nullptr || driver_path_env[0] == '\0') {
    throw std::runtime_error("DRIVER_PATH not set — cannot locate ODBC driver library");
  }
  std::ostringstream ss;
  ss << "DRIVER={" << driver_path_env << "};";
  ss << "SERVER=localhost;";
  ss << "PORT=" << wm.port() << ";";
  ss << "ACCOUNT=testaccount;";
  ss << "UID=testuser;";
  ss << "PROTOCOL=http;";
  ss << "AUTHENTICATOR=SNOWFLAKE_JWT;";
  ss << "PRIV_KEY_BASE64=" << test_utils::base64_encode(key_pem) << ";";
  return ss.str();
}

#endif  // WIREMOCK_CLIENT_HPP
