#include <windows.h>

#include <sstream>
#include <stdexcept>

#include "Subprocess.hpp"

struct Subprocess::Impl {
  HANDLE process_handle{INVALID_HANDLE_VALUE};
  HANDLE job_handle{INVALID_HANDLE_VALUE};

  void start(const std::string& program, const std::vector<std::string>& args) {
    job_handle = CreateJobObject(nullptr, nullptr);
    if (job_handle != nullptr) {
      JOBOBJECT_EXTENDED_LIMIT_INFORMATION job_info{};
      job_info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
      SetInformationJobObject(job_handle, JobObjectExtendedLimitInformation, &job_info, sizeof(job_info));
    }

    std::ostringstream cmd;
    cmd << "\"" << program << "\"";
    for (auto& a : args)
      cmd << " \"" << a << "\"";
    std::string cmd_line = cmd.str();

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
      throw std::runtime_error("CreateProcess failed for " + program + " (error " + std::to_string(GetLastError()) +
                               ")");
    }

    process_handle = pi.hProcess;
    CloseHandle(pi.hThread);

    if (job_handle != nullptr) {
      AssignProcessToJobObject(job_handle, process_handle);
    }
  }

  void stop() {
    if (process_handle != INVALID_HANDLE_VALUE) {
      TerminateProcess(process_handle, 0);
      WaitForSingleObject(process_handle, 5000);
      CloseHandle(process_handle);
      process_handle = INVALID_HANDLE_VALUE;
    }
    if (job_handle != INVALID_HANDLE_VALUE) {
      CloseHandle(job_handle);
      job_handle = INVALID_HANDLE_VALUE;
    }
  }
};

Subprocess::Subprocess(const std::string& program, const std::vector<std::string>& args)
    : impl_(std::make_unique<Impl>()) {
  impl_->start(program, args);
}

Subprocess::~Subprocess() {
  if (impl_) impl_->stop();
}

Subprocess::Subprocess(Subprocess&&) noexcept = default;
Subprocess& Subprocess::operator=(Subprocess&&) noexcept = default;
