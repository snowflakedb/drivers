#ifndef WIREMOCK_CLIENT_HPP
#define WIREMOCK_CLIENT_HPP

#include <picojson.h>

#include <cstdlib>
#include <fstream>
#include <optional>
#include <sstream>
#include <stdexcept>
#include <string>
#include <thread>
#include <vector>

#include "Subprocess.hpp"
#include "compatibility.hpp"
#include "platform.hpp"
#include "utils.hpp"

/// Cross-platform wrapper around the WireMock standalone JAR.
///
/// Starts WireMock as a subprocess, exposes admin API helpers, and
/// cleans up on destruction.
///
/// The default mode is a plain HTTP server matching by URL path. Pass
/// `Mode::ForwardProxy` to start WireMock with `--enable-browser-proxying`,
/// which lets it act as an HTTP forward proxy and match requests by their
/// path even when the driver sends absolute URLs (used by proxy tests).
class WiremockClient {
 public:
  enum class Mode {
    Server,
    ForwardProxy,
  };

  /// TLS protocol version the HTTPS listener is restricted to. Used by the
  /// TLS-version-enforcement integration tests: the driver connects to the
  /// HTTPS port and the handshake succeeds or fails purely on the version
  /// window it was configured with.
  enum class TlsVersion {
    Tls12,
    Tls13,
  };

  WiremockClient() : WiremockClient(Mode::Server) {}

  explicit WiremockClient(Mode mode) : mode_(mode) { init(); }

  /// Start WireMock with an HTTPS listener restricted to a single TLS protocol
  /// version (in addition to the plain-HTTP admin port used for health checks
  /// and mapping uploads). Connect the driver to `https_port()`.
  WiremockClient(Mode mode, TlsVersion tls_version) : mode_(mode), tls_version_(tls_version) { init(); }

  ~WiremockClient() {
    process_.reset();
    std::filesystem::remove_all(root_dir_);
  }

  WiremockClient(const WiremockClient&) = delete;
  WiremockClient& operator=(const WiremockClient&) = delete;

  std::string http_url() const { return "http://localhost:" + std::to_string(port_); }

  int port() const { return port_; }

  /// HTTPS listener port (only valid when constructed with a `TlsVersion`).
  int https_port() const { return https_port_; }

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
  Mode mode_;
  std::optional<TlsVersion> tls_version_;
  int port_;
  int https_port_{};
  std::filesystem::path root_dir_;
  std::unique_ptr<Subprocess> process_;

  void init() {
    // WireMock binds its port in a separate process, but find_free_port() must
    // close its probe socket before that happens — so under parallel test
    // execution (ctest -j) another process can steal the port between the two.
    // The socket can't be held open across the hand-off, so make bring-up
    // self-healing: on a failed start, tear down and retry with a fresh port.
    constexpr int kMaxAttempts = 5;
    for (int attempt = 1; attempt <= kMaxAttempts; ++attempt) {
      port_ = platform::find_free_port();
      if (tls_version_) {
        https_port_ = platform::find_free_port();
      }
      start_process();
      if (wait_for_health() && (!tls_version_ || wait_for_https_ready())) {
        return;
      }
      // Lost the port race, WireMock failed to bind, or it died before becoming
      // ready. Discard this attempt and retry with a freshly allocated port.
      process_.reset();
      std::error_code ec;
      std::filesystem::remove_all(root_dir_, ec);
    }
    throw std::runtime_error("WireMock did not become healthy after " + std::to_string(kMaxAttempts) + " attempts");
  }

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

  static std::filesystem::path wiremock_keystore_path() {
    return test_utils::repo_root() / "tests" / "wiremock" / "wiremock-keystore.p12";
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

    std::vector<std::string> args;
    // JVM options must precede `-jar`. To restrict the HTTPS listener to a
    // single TLS version we disable the others JVM-wide via
    // `jdk.tls.disabledAlgorithms` (WireMock/Jetty has no CLI flag for this).
    if (tls_version_) {
      args.push_back("-Djava.security.properties=" + write_tls_security_override());
    }
    args.insert(args.end(), {"-jar", jar.string(), "--root-dir", root_dir_.string(), "--port", port_str,
                             "--proxy-pass-through", "false", "--disable-gzip"});
    if (tls_version_) {
      // Plain-HTTP admin port stays up (used for health + mapping uploads); the
      // driver connects to this HTTPS port, which serves the same stubs.
      // A custom PKCS12 keystore with CN=localhost and SAN=localhost is used so
      // the old Simba ODBC driver passes hostname verification (the built-in
      // WireMock cert has CN=Tom Akehurst, which fails hostname checks against
      // SERVER=localhost in any libcurl-backed driver).
      auto keystore = wiremock_keystore_path();
      if (!std::filesystem::exists(keystore)) {
        throw std::runtime_error("WireMock keystore not found at: " + keystore.string());
      }
      args.emplace_back("--https-port");
      args.emplace_back(std::to_string(https_port_));
      args.emplace_back("--https-keystore");
      args.emplace_back(keystore.string());
      args.emplace_back("--keystore-type");
      args.emplace_back("PKCS12");
      args.emplace_back("--keystore-password");
      args.emplace_back("password");
    }
    if (mode_ == Mode::ForwardProxy) {
      args.emplace_back("--enable-browser-proxying");
    }
    process_ = std::make_unique<Subprocess>("java", std::move(args));
  }

  /// Writes a `java.security` override that disables every TLS protocol except
  /// the selected one, so WireMock's HTTPS port offers only that version.
  /// Returns the file path for `-Djava.security.properties` (single `=`
  /// overrides just this property, leaving the JDK's other defaults intact).
  std::string write_tls_security_override() const {
    const std::string disabled =
        (*tls_version_ == TlsVersion::Tls12) ? "SSLv3, TLSv1, TLSv1.1, TLSv1.3" : "SSLv3, TLSv1, TLSv1.1, TLSv1.2";
    auto path = root_dir_ / "tls.security";
    std::ofstream f(path);
    f << "jdk.tls.disabledAlgorithms=" << disabled << "\n";
    return path.string();
  }

  /// Polls the plain-HTTP admin health endpoint. Returns true once WireMock is
  /// healthy, false if it does not become healthy within the timeout or the
  /// process exits first (a failed port bind). Never throws — `init()` decides
  /// whether to retry.
  bool wait_for_health(int timeout_secs = 15) const {
    for (int i = 0; i < timeout_secs * 5; ++i) {
      if (!process_->running()) return false;  // bind failed / crashed — don't burn the full timeout
      std::string cmd =
          "curl -s -o " + platform::null_device() + " -w \"%{http_code}\" " + admin_url("/__admin/health");
      std::string code = platform::exec_command(cmd);
      if (code == "200") return true;
      std::this_thread::sleep_for(std::chrono::milliseconds(200));
    }
    return false;
  }

  /// Waits until the HTTPS connector accepts a TLS connection. `wait_for_health`
  /// only proves the plain-HTTP admin port is up; without this a test could
  /// connect to `https_port_` before Jetty finishes binding it. Any HTTP
  /// response (even non-200) means the listener is bound; curl code "000" means
  /// the connection was refused. `-k`: the listener uses a self-signed test cert.
  bool wait_for_https_ready(int timeout_secs = 15) const {
    const std::string url = "https://localhost:" + std::to_string(https_port_) + "/__admin/health";
    for (int i = 0; i < timeout_secs * 5; ++i) {
      if (!process_->running()) return false;
      std::string cmd = "curl -sk -o " + platform::null_device() + " -w \"%{http_code}\" " + url;
      std::string code = platform::exec_command(cmd);
      if (code != "000") return true;
      std::this_thread::sleep_for(std::chrono::milliseconds(200));
    }
    return false;
  }
};

#endif  // WIREMOCK_CLIENT_HPP
