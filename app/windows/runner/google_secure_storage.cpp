#include "google_secure_storage.h"

#include <windows.h>
#include <dpapi.h>
#include <shlobj.h>

#include <fstream>
#include <iterator>
#include <stdexcept>
#include <vector>

namespace afudesk {
namespace {
constexpr size_t kMaxPlaintext = 65536;
constexpr size_t kMaxEncrypted = kMaxPlaintext + 16384;

struct ProtectedBlob {
  DATA_BLOB data{};
  ~ProtectedBlob() {
    if (data.pbData) {
      SecureZeroMemory(data.pbData, data.cbData);
      LocalFree(data.pbData);
    }
  }
};

void Require(bool ok) {
  if (!ok) throw std::runtime_error("secure_storage");
}
}  // namespace

std::filesystem::path GoogleSessionPath() {
  PWSTR local_app_data = nullptr;
  Require(SUCCEEDED(SHGetKnownFolderPath(FOLDERID_LocalAppData, 0, nullptr, &local_app_data)));
  std::filesystem::path path(local_app_data);
  CoTaskMemFree(local_app_data);
  return path / L"AfuDesk" / L"google-session.dpapi";
}

void WriteGoogleSession(const std::filesystem::path& path, const std::string& value) {
  Require(!value.empty() && value.size() <= kMaxPlaintext);
  DATA_BLOB input{static_cast<DWORD>(value.size()),
                  reinterpret_cast<BYTE*>(const_cast<char*>(value.data()))};
  ProtectedBlob encrypted;
  // Current-user scope only; NEVER use CRYPTPROTECT_LOCAL_MACHINE.
  Require(CryptProtectData(&input, L"AfuDesk session v1", nullptr, nullptr,
                          nullptr, CRYPTPROTECT_UI_FORBIDDEN, &encrypted.data) != FALSE);
  std::filesystem::create_directories(path.parent_path());
  auto temporary = path;
  temporary += L".tmp";
  std::ofstream output(temporary, std::ios::binary | std::ios::trunc);
  Require(output.is_open());
  output.write(reinterpret_cast<const char*>(encrypted.data.pbData), encrypted.data.cbData);
  output.flush();
  Require(output.good());
  output.close();
  Require(!output.fail());
  Require(MoveFileExW(temporary.c_str(), path.c_str(),
                      MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH) != FALSE);
}

std::optional<std::string> ReadGoogleSession(const std::filesystem::path& path) {
  if (!std::filesystem::exists(path)) return std::nullopt;
  const auto size = std::filesystem::file_size(path);
  Require(size > 0 && size <= kMaxEncrypted);
  std::ifstream input_file(path, std::ios::binary);
  Require(input_file.is_open());
  std::vector<BYTE> bytes(static_cast<size_t>(size));
  input_file.read(reinterpret_cast<char*>(bytes.data()), static_cast<std::streamsize>(bytes.size()));
  Require(input_file.good());
  DATA_BLOB input{static_cast<DWORD>(bytes.size()), bytes.data()};
  ProtectedBlob plaintext;
  Require(CryptUnprotectData(&input, nullptr, nullptr, nullptr, nullptr,
                            CRYPTPROTECT_UI_FORBIDDEN, &plaintext.data) != FALSE);
  Require(plaintext.data.cbData > 0 && plaintext.data.cbData <= kMaxPlaintext);
  return std::string(reinterpret_cast<const char*>(plaintext.data.pbData), plaintext.data.cbData);
}

void ClearGoogleSession(const std::filesystem::path& path) {
  // Also remove an interrupted atomic write; never retain a second usable bundle.
  auto temporary = path;
  temporary += L".tmp";
  std::error_code temporary_error;
  std::error_code session_error;
  std::filesystem::remove(temporary, temporary_error);
  std::filesystem::remove(path, session_error);
  Require(!temporary_error && !session_error);
}
}  // namespace afudesk
