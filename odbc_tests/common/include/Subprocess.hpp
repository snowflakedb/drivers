#ifndef SUBPROCESS_HPP
#define SUBPROCESS_HPP

#include <memory>
#include <string>
#include <vector>

/// RAII wrapper around a background subprocess.
///
/// Starts the given program with arguments, suppressing stdout/stderr.
/// The destructor terminates the process and waits for it to exit.
class Subprocess {
 public:
  Subprocess(const std::string& program, const std::vector<std::string>& args);
  ~Subprocess();

  Subprocess(const Subprocess&) = delete;
  Subprocess& operator=(const Subprocess&) = delete;
  Subprocess(Subprocess&&) noexcept;
  Subprocess& operator=(Subprocess&&) noexcept;

  /// Returns true while the child is still running. If the child has exited it
  /// is reaped here so the destructor does not block; subsequent calls return
  /// false. Used to fail fast when a launched process dies during startup.
  bool running() const;

 private:
  struct Impl;
  std::unique_ptr<Impl> impl_;
};

#endif  // SUBPROCESS_HPP
