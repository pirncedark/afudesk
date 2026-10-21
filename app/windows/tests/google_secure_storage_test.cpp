#include "../runner/google_secure_storage.h"
#include <windows.h>
#include <fstream>
#include <iostream>
#include <iterator>
#include <stdexcept>

static void Check(bool ok) {
  if (!ok) throw std::runtime_error("check failed");
}

int main() {
  // A unique test directory; never reads/writes the real session path.
  const auto directory = std::filesystem::temp_directory_path() /
      (L"afudesk-storage-test-" + std::to_wstring(GetCurrentProcessId()));
  const auto path = directory / L"session.dpapi";
  try {
    Check(!afudesk::ReadGoogleSession(path).has_value());
    const std::string secret = "test-only-session-123456789";
    afudesk::WriteGoogleSession(path, secret);
    Check(afudesk::ReadGoogleSession(path) == secret);
    std::ifstream input(path, std::ios::binary);
    const std::string encrypted((std::istreambuf_iterator<char>(input)), {});
    input.close();
    Check(encrypted.find(secret) == std::string::npos);
    afudesk::WriteGoogleSession(path, "replacement");
    Check(afudesk::ReadGoogleSession(path) == "replacement");
    std::ofstream corrupt(path, std::ios::binary | std::ios::trunc);
    corrupt << "invalid-dpapi-blob";
    corrupt.close();
    bool rejected = false;
    try { (void)afudesk::ReadGoogleSession(path); } catch (...) { rejected = true; }
    Check(rejected);
    rejected = false;
    try { afudesk::WriteGoogleSession(path, std::string(65537, 'x')); } catch (...) { rejected = true; }
    Check(rejected);
    afudesk::ClearGoogleSession(path);
    afudesk::ClearGoogleSession(path);
    Check(!afudesk::ReadGoogleSession(path).has_value());
    std::filesystem::remove(directory);
    std::cout << "PASS: missing, roundtrip, ciphertext, overwrite, tamper, size, clear\n";
    return 0;
  } catch (const std::exception&) {
    std::error_code ignored;
    std::filesystem::remove_all(directory, ignored);
    std::cerr << "FAIL: secure storage\n";
    return 1;
  }
}
