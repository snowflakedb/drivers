#include <fcntl.h>
#include <sys/wait.h>
#include <unistd.h>

#include <csignal>
#include <stdexcept>

#include "Subprocess.hpp"

struct Subprocess::Impl {
  pid_t pid{-1};

  void start(const std::string& program, const std::vector<std::string>& args) {
    pid = fork();
    if (pid < 0) {
      throw std::runtime_error("fork() failed for subprocess: " + program);
    }

    if (pid == 0) {
      setsid();
      int dev_null = open("/dev/null", O_WRONLY);
      if (dev_null >= 0) {
        dup2(dev_null, STDOUT_FILENO);
        dup2(dev_null, STDERR_FILENO);
        close(dev_null);
      }

      std::vector<const char*> argv;
      argv.push_back(program.c_str());
      for (auto& a : args)
        argv.push_back(a.c_str());
      argv.push_back(nullptr);

      execvp(program.c_str(), const_cast<char* const*>(argv.data()));
      _exit(1);
    }
  }

  void stop() {
    if (pid > 0) {
      kill(-pid, SIGTERM);
      waitpid(pid, nullptr, 0);
      pid = -1;
    }
  }

  bool running() {
    if (pid <= 0) return false;
    int status = 0;
    pid_t r = waitpid(pid, &status, WNOHANG);
    if (r == 0) return true;  // still alive
    // Exited (r == pid) or already gone (r == -1): reap and mark so stop()
    // does not block on an already-collected child.
    pid = -1;
    return false;
  }
};

Subprocess::Subprocess(const std::string& program, const std::vector<std::string>& args)
    : impl_(std::make_unique<Impl>()) {
  impl_->start(program, args);
}

Subprocess::~Subprocess() {
  if (impl_) impl_->stop();
}

bool Subprocess::running() const { return impl_ && impl_->running(); }

Subprocess::Subprocess(Subprocess&&) noexcept = default;
Subprocess& Subprocess::operator=(Subprocess&&) noexcept = default;
