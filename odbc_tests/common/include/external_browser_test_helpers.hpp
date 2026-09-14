#ifndef EXTERNAL_BROWSER_TEST_HELPERS_HPP
#define EXTERNAL_BROWSER_TEST_HELPERS_HPP

#ifdef _WIN32
#include <winsock2.h>
#else
#include <sys/socket.h>
#include <unistd.h>

#include <arpa/inet.h>
#include <netinet/in.h>
#endif

#include <picojson.h>

#include <chrono>
#include <cstdint>
#include <sstream>
#include <stdexcept>
#include <string>
#include <thread>

#include "WiremockClient.hpp"
#include "test_setup.hpp"

namespace external_browser_test {

#ifdef _WIN32
using socket_t = SOCKET;
constexpr socket_t kInvalidSocket = INVALID_SOCKET;
inline void close_socket(socket_t sock) { ::closesocket(sock); }
#else
using socket_t = int;
constexpr socket_t kInvalidSocket = -1;
inline void close_socket(socket_t sock) { ::close(sock); }
#endif

class ScopedSocket {
 public:
  explicit ScopedSocket(socket_t sock) : sock_(sock) {}
  ~ScopedSocket() {
    if (sock_ != kInvalidSocket) {
      close_socket(sock_);
    }
  }

  ScopedSocket(const ScopedSocket&) = delete;
  ScopedSocket& operator=(const ScopedSocket&) = delete;

  socket_t get() const { return sock_; }

 private:
  socket_t sock_;
};

inline std::string get_external_browser_connection_string(const WiremockClient& wm,
                                                          const std::string& user = "test_user") {
  std::ostringstream ss;
  configure_driver_string(ss);
  ss << "SERVER=localhost;";
  ss << "PORT=" << wm.port() << ";";
  ss << "ACCOUNT=testaccount;";
  ss << "UID=" << user << ";";
  ss << "AUTHENTICATOR=EXTERNALBROWSER;";
  ss << "SSL=off;";
  ss << "DisableOCSPCheck=true;";
  return ss.str();
}

/// Poll WireMock for the authenticator-request, extract the redirect port,
/// then send a fake token to sf_core's localhost callback listener.
inline void simulate_browser_callback(const WiremockClient& wm, const std::string& token, int timeout_ms = 10000) {
  auto deadline = std::chrono::steady_clock::now() + std::chrono::milliseconds(timeout_ms);
  while (std::chrono::steady_clock::now() < deadline) {
    auto requests = wm.find_requests("/session/authenticator-request.*");
    if (!requests.empty()) {
      const auto& req_obj = requests[0].get<picojson::object>();
      auto body_it = req_obj.find("body");
      if (body_it == req_obj.end() || !body_it->second.is<std::string>()) {
        throw std::runtime_error("authenticator-request has no body string");
      }

      picojson::value body_json;
      std::string err = picojson::parse(body_json, body_it->second.get<std::string>());
      if (!err.empty()) {
        throw std::runtime_error("Failed to parse authenticator-request body: " + err);
      }

      auto port_str = body_json.get<picojson::object>()["data"]
                          .get<picojson::object>()["BROWSER_MODE_REDIRECT_PORT"]
                          .get<std::string>();
      int port = std::stoi(port_str);

#ifdef _WIN32
      WSADATA wsa_data;
      WSAStartup(MAKEWORD(2, 2), &wsa_data);
      ScopedSocket sock(::socket(AF_INET, SOCK_STREAM, IPPROTO_TCP));
#else
      ScopedSocket sock(::socket(AF_INET, SOCK_STREAM, 0));
#endif
      if (sock.get() == kInvalidSocket) {
        throw std::runtime_error("Failed to create socket for the browser callback");
      }

      sockaddr_in addr{};
      addr.sin_family = AF_INET;
      addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
      addr.sin_port = htons(static_cast<uint16_t>(port));

      if (::connect(sock.get(), reinterpret_cast<sockaddr*>(&addr), sizeof(addr)) < 0) {
        throw std::runtime_error("Failed to connect to callback listener on port " + port_str);
      }

      std::string http_request = "GET /?token=" + token + " HTTP/1.1\r\nHost: localhost\r\n\r\n";
      char buf[4096];
#ifdef _WIN32
      ::send(sock.get(), http_request.c_str(), static_cast<int>(http_request.size()), 0);
      ::recv(sock.get(), buf, sizeof(buf), 0);
#else
      ::send(sock.get(), http_request.c_str(), http_request.size(), 0);
      ::recv(sock.get(), buf, sizeof(buf), 0);
#endif
      return;
    }
    std::this_thread::sleep_for(std::chrono::milliseconds(200));
  }
  throw std::runtime_error("authenticator-request never arrived at WireMock");
}

}  // namespace external_browser_test

#endif
