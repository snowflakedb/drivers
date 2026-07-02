#ifndef PLATFORM_HPP
#define PLATFORM_HPP

#include <cstdio>
#include <sstream>
#include <stdexcept>
#include <string>

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <winsock2.h>
#include <ws2tcpip.h>
#pragma comment(lib, "ws2_32.lib")
#else
#include <sys/socket.h>
#include <unistd.h>

#include <netinet/in.h>
#endif

namespace platform {

inline std::string null_device() {
#ifdef _WIN32
  return "NUL";
#else
  return "/dev/null";
#endif
}

inline std::string null_redirect() { return " > " + null_device() + " 2>&1"; }

inline std::string exec_command(const std::string& cmd) {
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

inline int find_free_port() {
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

}  // namespace platform

#endif  // PLATFORM_HPP
