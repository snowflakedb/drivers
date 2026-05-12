#ifndef WIREMOCK_CLIENT_HPP
#define WIREMOCK_CLIENT_HPP

#include <picojson.h>

#include <cstdlib>
#include <fstream>
#include <sstream>
#include <stdexcept>
#include <string>
#include <thread>
#include <vector>

#include "Subprocess.hpp"
#include "platform.hpp"
#include "utils.hpp"

/// Cross-platform wrapper around the WireMock standalone JAR.
///
/// Starts WireMock as a subprocess, exposes admin API helpers, and
/// cleans up on destruction.
class WiremockClient {
 public:
  WiremockClient() {
    port_ = platform::find_free_port();
    start_process();
    wait_for_health();
  }

  ~WiremockClient() {
    process_.reset();
    std::filesystem::remove_all(root_dir_);
  }

  WiremockClient(const WiremockClient&) = delete;
  WiremockClient& operator=(const WiremockClient&) = delete;

  std::string http_url() const { return "http://localhost:" + std::to_string(port_); }

  int port() const { return port_; }

  void add_mapping_file(const std::string& relative_path) {
    auto file_path = wiremock_mappings_dir() / relative_path;
    if (!std::filesystem::exists(file_path)) {
      throw std::runtime_error("WireMock mapping file not found: " + file_path.string());
    }

    std::ifstream ifs(file_path);
    std::string content((std::istreambuf_iterator<char>(ifs)), std::istreambuf_iterator<char>());

    picojson::value json;
    std::string err = picojson::parse(json, content);
    if (!err.empty()) {
      throw std::runtime_error("Failed to parse mapping file " + relative_path + ": " + err);
    }

    if (json.is<picojson::object>() && json.get<picojson::object>().count("mappings")) {
      auto& arr = json.get<picojson::object>().at("mappings").get<picojson::array>();
      for (auto& mapping : arr) {
        post_mapping(mapping.serialize(), relative_path);
      }
    } else {
      post_mapping(content, relative_path);
    }
  }

  void add_catch_all() {
    auto tmp = tmp_file("catch_all");
    {
      std::ofstream f(tmp);
      f << R"({
        "request": {"method": "ANY", "urlPattern": ".*"},
        "response": {
          "status": 200,
          "headers": {"Content-Type": "application/json"},
          "jsonBody": {"success": true, "data": {}}
        },
        "priority": 999
      })";
    }
    std::string cmd = curl_post("/__admin/mappings", tmp) + platform::null_redirect();
    if (std::system(cmd.c_str()) != 0) {
      std::filesystem::remove(tmp);
      throw std::runtime_error("Failed to add WireMock catch-all mapping");
    }
    std::filesystem::remove(tmp);
  }

  int get_request_count(const std::string& method, const std::string& url_path) const {
    std::string body = R"({"method":")" + method + R"(","urlPath":")" + url_path + R"("})";

    auto tmp = tmp_file("request_count");
    {
      std::ofstream f(tmp);
      f << body;
    }
    std::string cmd = curl_post("/__admin/requests/count", tmp);
    std::string response = platform::exec_command(cmd);
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

  /// Find requests matching a URL path pattern and return their bodies as parsed JSON.
  std::vector<picojson::value> find_requests(const std::string& url_path_pattern) const {
    std::string body = R"({"urlPathPattern":")" + url_path_pattern + R"("})";

    auto tmp = tmp_file("find_requests");
    {
      std::ofstream f(tmp);
      f << body;
    }
    std::string cmd = curl_post("/__admin/requests/find", tmp);
    std::string response = platform::exec_command(cmd);
    std::filesystem::remove(tmp);

    picojson::value json;
    std::string err = picojson::parse(json, response);
    if (!err.empty() || !json.is<picojson::object>()) {
      throw std::runtime_error("WireMock find_requests parse error: " + err + " | body: " + response);
    }

    std::vector<picojson::value> results;
    const auto& obj = json.get<picojson::object>();
    auto it = obj.find("requests");
    if (it != obj.end() && it->second.is<picojson::array>()) {
      for (const auto& req : it->second.get<picojson::array>()) {
        results.push_back(req);
      }
    }
    return results;
  }

 private:
  int port_;
  std::filesystem::path root_dir_;
  std::unique_ptr<Subprocess> process_;

  std::string admin_url(const std::string& path) const { return "http://localhost:" + std::to_string(port_) + path; }

  std::filesystem::path tmp_file(const std::string& label) const {
    return std::filesystem::temp_directory_path() / ("wm_" + label + "_" + std::to_string(port_) + ".json");
  }

  std::string curl_post(const std::string& endpoint, const std::filesystem::path& data_file) const {
    return "curl -s -X POST " + admin_url(endpoint) +
           " -H \"Content-Type: application/json\""
           " --data-binary \"@" +
           data_file.string() + "\"";
  }

  void post_mapping(const std::string& body, const std::string& source_name) {
    auto tmp = tmp_file("mapping");
    {
      std::ofstream f(tmp);
      f << body;
    }
    std::string cmd = curl_post("/__admin/mappings", tmp) + platform::null_redirect();
    int rc = std::system(cmd.c_str());
    std::filesystem::remove(tmp);
    if (rc != 0) {
      throw std::runtime_error("Failed to add WireMock mapping: " + source_name);
    }
  }

  static std::filesystem::path wiremock_mappings_dir() {
    return test_utils::repo_root() / "tests" / "wiremock" / "mappings";
  }

  static std::filesystem::path wiremock_jar_path() {
    return test_utils::repo_root() / "tests" / "wiremock" / "wiremock_standalone" / "wiremock-standalone-3.13.2.jar";
  }

  void start_process() {
    auto jar = wiremock_jar_path();
    if (!std::filesystem::exists(jar)) {
      throw std::runtime_error("WireMock JAR not found at: " + jar.string());
    }

    std::string port_str = std::to_string(port_);
    root_dir_ = std::filesystem::temp_directory_path() / ("wm_root_" + port_str);
    std::filesystem::create_directories(root_dir_ / "mappings");
    std::filesystem::create_directories(root_dir_ / "__files");

    process_ = std::make_unique<Subprocess>(
        "java", std::vector<std::string>{"-jar", jar.string(), "--root-dir", root_dir_.string(), "--port", port_str,
                                         "--proxy-pass-through", "false", "--disable-gzip"});
  }

  void wait_for_health(int timeout_secs = 15) const {
    for (int i = 0; i < timeout_secs * 5; ++i) {
      std::string cmd =
          "curl -s -o " + platform::null_device() + " -w \"%{http_code}\" " + admin_url("/__admin/health");
      std::string code = platform::exec_command(cmd);
      if (code == "200") return;
      std::this_thread::sleep_for(std::chrono::milliseconds(200));
    }
    throw std::runtime_error("WireMock did not become healthy within " + std::to_string(timeout_secs) + "s");
  }
};

/// Build an ODBC connection string pointing to a running WireMock instance.
inline std::string get_wiremock_connection_string(const WiremockClient& wm) {
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
  ss << "PWD=testpass;";
  ss << "SSL=off;";
  ss << "DisableOCSPCheck=true;";
  return ss.str();
}

#endif  // WIREMOCK_CLIENT_HPP
