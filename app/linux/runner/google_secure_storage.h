#ifndef AFUDESK_GOOGLE_SECURE_STORAGE_H_
#define AFUDESK_GOOGLE_SECURE_STORAGE_H_
#include <glib.h>
namespace afudesk {
// The caller owns the returned password and must use secret_password_free.
gchar* ReadGoogleSession(GError** error);
gboolean WriteGoogleSession(const gchar* value, GError** error);
gboolean ClearGoogleSession(GError** error);
}
#endif
