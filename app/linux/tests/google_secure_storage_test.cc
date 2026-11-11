#include "../runner/google_secure_storage.h"
#include <libsecret/secret.h>
#include <cstring>
#include <iostream>
#include <sys/stat.h>
#include <sys/wait.h>
#include <unistd.h>
#include <fstream>
#include <string>

int main(int argc, char** argv) {
  g_autoptr(GError) error = nullptr;
  if (argc > 1 && std::strcmp(argv[1], "--concurrent") == 0) {
    pid_t children[4] {};
    for (int i = 0; i < 4; ++i) {
      children[i] = fork();
      if (children[i] < 0) return 30;
      if (children[i] == 0) {
        for (int j = 0; j < 5; ++j) {
          if (!afudesk::WriteGoogleSession("concurrent-session", &error) || error != nullptr) _exit(31);
          gchar* value = afudesk::ReadGoogleSession(&error);
          const bool ok = error == nullptr && value != nullptr && std::strcmp(value, "concurrent-session") == 0;
          if (value != nullptr) secret_password_free(value);
          if (!ok) _exit(32);
        }
        _exit(0);
      }
    }
    for (pid_t child : children) {
      int status = 0;
      if (waitpid(child, &status, 0) != child || !WIFEXITED(status) || WEXITSTATUS(status) != 0) return 33;
    }
    std::cout << "PASS: four processes share atomic key creation and encrypted writes\n";
    return 0;
  }
  if (argc > 1 && std::strcmp(argv[1], "--sticky") == 0) {
    const SecretSchema schema = {"com.afu.afudesk.GoogleSession", SECRET_SCHEMA_NONE,
      {{"slot", SECRET_SCHEMA_ATTRIBUTE_STRING}, {nullptr, SECRET_SCHEMA_ATTRIBUTE_STRING}},
      0, nullptr, nullptr, nullptr, nullptr, nullptr, nullptr, nullptr};
    if (!secret_password_store_sync(&schema, SECRET_COLLECTION_DEFAULT, "CI stale session",
        "stale-keyring-session", nullptr, &error, "slot", "google-session", nullptr)) return 40;
    gchar* value = afudesk::ReadGoogleSession(&error);
    if (value != nullptr || error != nullptr) return 41;
    if (!afudesk::WriteGoogleSession("current-fallback-session", &error) || error != nullptr) return 42;
    value = afudesk::ReadGoogleSession(&error);
    if (error != nullptr || value == nullptr || std::strcmp(value, "current-fallback-session") != 0) return 43;
    secret_password_free(value);
    value = secret_password_lookup_sync(&schema, nullptr, &error, "slot", "google-session", nullptr);
    if (error != nullptr || value == nullptr || std::strcmp(value, "stale-keyring-session") != 0) return 44;
    secret_password_free(value);
    if (!afudesk::ClearGoogleSession(&error) || error != nullptr) return 45;
    value = afudesk::ReadGoogleSession(&error);
    if (value != nullptr || error != nullptr) return 46;
    std::cout << "PASS: returning Secret Service cannot resurrect or replace sticky fallback\n";
    return 0;
  }
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
    corrupt.seekg(-1, std::ios::end);
    char last = 0;
    corrupt.get(last);
    corrupt.seekp(-1, std::ios::end);
    corrupt.put(static_cast<char>(last ^ 1));
    corrupt.close();
    value = afudesk::ReadGoogleSession(&error);
    if (value != nullptr || error == nullptr) return 17;
    g_clear_error(&error);
    if (chmod((dir + "/key").c_str(), 0644) != 0) return 20;
    value = afudesk::ReadGoogleSession(&error);
    if (value != nullptr || error == nullptr) return 21;
    g_clear_error(&error);
    if (chmod((dir + "/key").c_str(), 0600) != 0) return 22;
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
