#include "../runner/google_secure_storage.h"
#include <libsecret/secret.h>
#include <cstring>
#include <iostream>
#include <sys/stat.h>
#include <fstream>
#include <string>

int main(int argc, char**) {
  g_autoptr(GError) error = nullptr;
  if (argc > 1) {
    if (!afudesk::ClearGoogleSession(&error) || error != nullptr) return 10;
    if (!afudesk::WriteGoogleSession("fallback-ci-session", &error) || error != nullptr) return 11;
    gchar* value = afudesk::ReadGoogleSession(&error);
    if (error != nullptr || value == nullptr || std::strcmp(value, "fallback-ci-session") != 0) return 12;
    secret_password_free(value);
    const std::string dir = std::string(g_get_user_data_dir()) + "/AfuDesk/secure-session";
    for (const auto& name : {"key", "session"}) {
      struct stat st {};
      if (stat((dir + "/" + name).c_str(), &st) != 0 || (st.st_mode & 0777) != 0600) return 13;
    }
    std::ifstream saved(dir + "/session", std::ios::binary);
    const std::string encrypted((std::istreambuf_iterator<char>(saved)), {});
    if (encrypted.find("fallback-ci-session") != std::string::npos) return 14;
    if (!afudesk::WriteGoogleSession("rotated-ci-session", &error) || error != nullptr) return 15;
    value = afudesk::ReadGoogleSession(&error);
    if (error != nullptr || value == nullptr || std::strcmp(value, "rotated-ci-session") != 0) return 16;
    secret_password_free(value);
    std::fstream corrupt(dir + "/session", std::ios::binary | std::ios::in | std::ios::out);
    corrupt.seekp(-1, std::ios::end);
    corrupt.put(static_cast<char>(encrypted.back() ^ 1));
    corrupt.close();
    value = afudesk::ReadGoogleSession(&error);
    if (value != nullptr || error == nullptr) return 17;
    g_clear_error(&error);
    if (!afudesk::ClearGoogleSession(&error) || error != nullptr) return 18;
    value = afudesk::ReadGoogleSession(&error);
    if (value != nullptr || error != nullptr) return 19;
    std::cout << "PASS: absent Secret Service uses encrypted 0600 storage; rotation, tamper rejection, clear\n";
    return 0;
  }
  if (!afudesk::ClearGoogleSession(&error) || error != nullptr) return 1;
  if (!afudesk::WriteGoogleSession("afudesk-ci-disposable-session", &error) || error != nullptr) return 2;
  gchar* value = afudesk::ReadGoogleSession(&error);
  const bool matches = error == nullptr && value != nullptr &&
      std::strcmp(value, "afudesk-ci-disposable-session") == 0;
  if (value != nullptr) secret_password_free(value);
  if (!matches) return 3;
  if (!afudesk::ClearGoogleSession(&error) || error != nullptr) return 4;
  value = afudesk::ReadGoogleSession(&error);
  const bool absent = error == nullptr && value == nullptr;
  if (value != nullptr) secret_password_free(value);
  if (!absent) return 5;
  std::cout << "PASS: Secret Service write/read/clear\n";
  return 0;
}
