#ifndef UTILS_HPP
#define UTILS_HPP

#include <array>
#include <cctype>
#include <filesystem>
#include <stdexcept>
#include <string>

#include <openssl/bio.h>
#include <openssl/evp.h>
#include <openssl/pem.h>

#ifdef _WIN32
#include <io.h>
#define popen _popen
#define pclose _pclose
#else
#include <cstdio>
#endif

namespace test_utils {

/// Base64-encode a string using OpenSSL's EVP_EncodeBlock.
inline std::string base64_encode(const std::string& input) {
  // EVP_EncodeBlock output size: 4 * ceil(n/3) + 1 (for NUL)
  std::string encoded(4 * ((input.size() + 2) / 3) + 1, '\0');
  int len = EVP_EncodeBlock(reinterpret_cast<unsigned char*>(encoded.data()),
                            reinterpret_cast<const unsigned char*>(input.data()), static_cast<int>(input.size()));
  encoded.resize(len);
  return encoded;
}

inline std::filesystem::path repo_root() {
  const char* git_root_env_value = std::getenv("GIT_ROOT");
  if (git_root_env_value != nullptr && git_root_env_value[0] != '\0') {
    return std::filesystem::path(git_root_env_value);
  }
  const char* cmd = "git rev-parse --show-toplevel";
#ifdef _WIN32
  FILE* pipe = _popen(cmd, "r");
#else
  FILE* pipe = popen(cmd, "r");
#endif
  if (!pipe) {
    throw std::runtime_error("Failed to determine repository root: unable to start git command");
  }

  std::array<char, 256> buffer{};
  std::string output;
  while (fgets(buffer.data(), static_cast<int>(buffer.size()), pipe) != nullptr) {
    output.append(buffer.data());
  }

#ifdef _WIN32
  int rc = _pclose(pipe);
#else
  int rc = pclose(pipe);
#endif

  while (!output.empty() && std::isspace(static_cast<unsigned char>(output.back()))) {
    output.pop_back();
  }

  if (rc == 0 && !output.empty()) {
    return std::filesystem::path(output);
  }

  throw std::runtime_error("Failed to determine repository root");
}

inline std::filesystem::path shared_test_data_dir() {
  return repo_root() / "tests" / "test_data" / "generated_test_data";
}

// Helper function to get test data file path
inline std::filesystem::path test_data_file_path(const std::string& relative_path) {
  return repo_root() / "tests" / "test_data" / relative_path;
}

/// Decrypt an encrypted PEM private key and write the unencrypted PEM to a file.
inline void decrypt_pem_key_to_file(const std::string& encrypted_pem, const std::string& password,
                                    const std::filesystem::path& output_path) {
  auto* bio_in = BIO_new_mem_buf(encrypted_pem.data(), static_cast<int>(encrypted_pem.size()));
  if (!bio_in) throw std::runtime_error("BIO_new_mem_buf failed");

  EVP_PKEY* pkey = PEM_read_bio_PrivateKey(bio_in, nullptr, nullptr, const_cast<char*>(password.c_str()));
  BIO_free(bio_in);
  if (!pkey) throw std::runtime_error("PEM_read_bio_PrivateKey failed — wrong password?");

  auto* bio_out = BIO_new_file(output_path.string().c_str(), "wb");
  if (!bio_out) {
    EVP_PKEY_free(pkey);
    throw std::runtime_error("BIO_new_file failed");
  }

  int rc = PEM_write_bio_PrivateKey(bio_out, pkey, nullptr, nullptr, 0, nullptr, nullptr);
  BIO_free(bio_out);
  EVP_PKEY_free(pkey);
  if (rc != 1) throw std::runtime_error("PEM_write_bio_PrivateKey failed");
}

/// Read an unencrypted PEM private key and write an encrypted PEM to a file.
inline void encrypt_pem_key_to_file(const std::string& unencrypted_pem, const std::string& password,
                                    const std::filesystem::path& output_path) {
  auto* bio_in = BIO_new_mem_buf(unencrypted_pem.data(), static_cast<int>(unencrypted_pem.size()));
  if (!bio_in) throw std::runtime_error("BIO_new_mem_buf failed");

  EVP_PKEY* pkey = PEM_read_bio_PrivateKey(bio_in, nullptr, nullptr, nullptr);
  BIO_free(bio_in);
  if (!pkey) throw std::runtime_error("PEM_read_bio_PrivateKey failed");

  auto* bio_out = BIO_new_file(output_path.string().c_str(), "wb");
  if (!bio_out) {
    EVP_PKEY_free(pkey);
    throw std::runtime_error("BIO_new_file failed");
  }

  int rc = PEM_write_bio_PrivateKey(bio_out, pkey, EVP_aes_256_cbc(), nullptr, 0, nullptr,
                                    const_cast<char*>(password.c_str()));
  BIO_free(bio_out);
  EVP_PKEY_free(pkey);
  if (rc != 1) throw std::runtime_error("PEM_write_bio_PrivateKey (encrypt) failed");
}

}  // namespace test_utils

#endif  // UTILS_HPP
