#include "../runner/google_secure_storage.h"
#include <libsecret/secret.h>
#include <cstring>
#include <iostream>

int main(int argc, char**) {
  g_autoptr(GError) error = nullptr;
  if (argc > 1) {
    // A private D-Bus without a Secret Service must fail closed.
    gchar* value = afudesk::ReadGoogleSession(&error);
    if (value != nullptr) secret_password_free(value);
    if (error == nullptr) return 1;
    std::cout << "PASS: missing Secret Service rejects storage\n";
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
