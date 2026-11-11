#include "google_secure_storage.h"
#include <libsecret/secret.h>

namespace {
const SecretSchema kSessionSchema = {
    "com.afu.afudesk.GoogleSession", SECRET_SCHEMA_NONE,
    {{"slot", SECRET_SCHEMA_ATTRIBUTE_STRING}, {nullptr, SECRET_SCHEMA_ATTRIBUTE_STRING}},
    0, nullptr, nullptr, nullptr, nullptr, nullptr, nullptr, nullptr};
}
namespace afudesk {
gchar* ReadGoogleSession(GError** error) {
  return secret_password_lookup_sync(&kSessionSchema, nullptr, error,
                                    "slot", "google-session", nullptr);
}
gboolean WriteGoogleSession(const gchar* value, GError** error) {
  return secret_password_store_sync(&kSessionSchema, SECRET_COLLECTION_DEFAULT,
      "AfuDesk Google session", value, nullptr, error,
      "slot", "google-session", nullptr);
}
gboolean ClearGoogleSession(GError** error) {
  secret_password_clear_sync(&kSessionSchema, nullptr, error,
                            "slot", "google-session", nullptr);
  // Missing records are already cleared; service failures remain errors.
  return *error == nullptr;
}
}
