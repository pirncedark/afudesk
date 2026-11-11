#include "google_secure_storage.h"
#include <libsecret/secret.h>
#include <glib/gstdio.h>
#include <openssl/evp.h>
#include <openssl/rand.h>
#include <openssl/crypto.h>
#include <sys/stat.h>
#include <sys/file.h>
#include <sys/syscall.h>
#include <linux/fs.h>
#include <fcntl.h>
#include <unistd.h>
#include <array>
#include <cerrno>
#include <cstring>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

namespace {
const SecretSchema kSessionSchema = {
    "com.afu.afudesk.GoogleSession", SECRET_SCHEMA_NONE,
    {{"slot", SECRET_SCHEMA_ATTRIBUTE_STRING}, {nullptr, SECRET_SCHEMA_ATTRIBUTE_STRING}},
    0, nullptr, nullptr, nullptr, nullptr, nullptr, nullptr, nullptr};

void Check(bool ok) {
  if (!ok) throw std::runtime_error("secure storage unavailable");
}
struct File {
  explicit File(int value) : fd(value) { Check(fd >= 0); }
  ~File() { close(fd); }
  File(const File&) = delete;
  File& operator=(const File&) = delete;
  int fd;
};
struct Key {
  std::array<unsigned char, 32> bytes {};
  ~Key() { OPENSSL_cleanse(bytes.data(), bytes.size()); }
};
struct Plaintext {
  std::vector<unsigned char> bytes;
  ~Plaintext() { OPENSSL_cleanse(bytes.data(), bytes.size()); }
};
void Error(GError** error) {
  g_set_error_literal(error, G_IO_ERROR, G_IO_ERROR_FAILED,
                     "Oturum güvenle kaydedilemedi; uygulamayı yeniden açıp dene.");
}
bool Unavailable(GError* error) {
  if (error == nullptr) return false;
  if (error->domain == G_IO_ERROR &&
      (error->code == G_IO_ERROR_NOT_FOUND || error->code == G_IO_ERROR_CONNECTION_REFUSED ||
       error->code == G_IO_ERROR_NOT_CONNECTED || error->code == G_IO_ERROR_CLOSED)) return true;
  if (error->domain == G_DBUS_ERROR &&
      (error->code == G_DBUS_ERROR_SERVICE_UNKNOWN || error->code == G_DBUS_ERROR_NAME_HAS_NO_OWNER ||
       error->code == G_DBUS_ERROR_NO_SERVER || error->code == G_DBUS_ERROR_FILE_NOT_FOUND)) return true;
  return false;  // Locked keyrings and denied permissions never trigger fallback.
}
int Directory() {
  const std::string base = std::string(g_get_user_data_dir()) + "/AfuDesk";
  Check(g_mkdir_with_parents(base.c_str(), 0700) == 0);
  File parent(open(base.c_str(), O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC));
  struct stat parent_stat {};
  Check(fstat(parent.fd, &parent_stat) == 0 && parent_stat.st_uid == geteuid() &&
        (parent_stat.st_mode & 0022) == 0);
  Check(mkdirat(parent.fd, "secure-session", 0700) == 0 || errno == EEXIST);
  const int fd = openat(parent.fd, "secure-session", O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
  if (fd < 0) throw std::runtime_error("private directory unavailable");
  struct stat st {};
  if (fstat(fd, &st) != 0 || st.st_uid != geteuid() || (st.st_mode & 0777) != 0700) {
    close(fd);
    throw std::runtime_error("private directory unavailable");
  }
  return fd;
}
void PrivateFile(int fd) {
  struct stat st {};
  Check(fstat(fd, &st) == 0 && S_ISREG(st.st_mode) && st.st_uid == geteuid() &&
        st.st_nlink == 1 && (st.st_mode & 0777) == 0600);
}
struct StoreLock {
  File directory;
  File file;
  StoreLock() : directory(Directory()), file(openat(directory.fd, ".lock", O_RDWR | O_CREAT | O_NOFOLLOW | O_CLOEXEC, 0600)) {
    PrivateFile(file.fd);
    for (int attempt = 0; attempt < 200; ++attempt) {
      if (flock(file.fd, LOCK_EX | LOCK_NB) == 0) return;
      Check(errno == EWOULDBLOCK || errno == EAGAIN || errno == EINTR);
      usleep(10000);
    }
    throw std::runtime_error("storage is busy");
  }
};
bool HasFile(int dir) {
  struct stat st {};
  if (fstatat(dir, "session", &st, AT_SYMLINK_NOFOLLOW) == 0) return true;
  Check(errno == ENOENT);
  return false;
}
void WriteAll(int fd, const unsigned char* data, size_t size) {
  while (size > 0) {
    const ssize_t n = write(fd, data, size);
    if (n < 0 && errno == EINTR) continue;
    Check(n > 0);
    data += n;
    size -= static_cast<size_t>(n);
  }
  Check(fsync(fd) == 0);
}
std::vector<unsigned char> ReadAll(int fd, size_t limit) {
  PrivateFile(fd);
  struct stat st {};
  Check(fstat(fd, &st) == 0 && st.st_size >= 0 && static_cast<size_t>(st.st_size) <= limit);
  std::vector<unsigned char> bytes(static_cast<size_t>(st.st_size));
  size_t pos = 0;
  while (pos < bytes.size()) {
    const ssize_t n = read(fd, bytes.data() + pos, bytes.size() - pos);
    if (n < 0 && errno == EINTR) continue;
    Check(n > 0);
    pos += static_cast<size_t>(n);
  }
  return bytes;
}
void LoadKey(int dir, Key& key, bool create) {
  int fd = openat(dir, "key", O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
  if (fd < 0 && errno == ENOENT && create) {
    std::array<unsigned char, 8> random {};
    Check(RAND_bytes(random.data(), random.size()) == 1);
    gchar* encoded = g_base64_encode(random.data(), random.size());
    std::string temporary = std::string("key-") + encoded;
    g_free(encoded);
    for (auto& c : temporary) if (c == '/' || c == '+') c = '_';
    File file(openat(dir, temporary.c_str(), O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, 0600));
    try {
      PrivateFile(file.fd);
      Check(RAND_bytes(key.bytes.data(), key.bytes.size()) == 1);
      WriteAll(file.fd, key.bytes.data(), key.bytes.size());
      // Publish a complete key atomically without replacing any existing key.
      const bool published = syscall(SYS_renameat2, dir, temporary.c_str(), dir, "key", RENAME_NOREPLACE) == 0;
      const int saved_errno = errno;
      if (!published) Check(unlinkat(dir, temporary.c_str(), 0) == 0);
      Check(fsync(dir) == 0);
      if (published) return;
      Check(saved_errno == EEXIST);
    } catch (...) { unlinkat(dir, temporary.c_str(), 0); throw; }
    fd = openat(dir, "key", O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
  }
  File file(fd);
  auto bytes = ReadAll(file.fd, key.bytes.size());
  Check(bytes.size() == key.bytes.size());
  std::memcpy(key.bytes.data(), bytes.data(), bytes.size());
  OPENSSL_cleanse(bytes.data(), bytes.size());
}
using Cipher = std::unique_ptr<EVP_CIPHER_CTX, decltype(&EVP_CIPHER_CTX_free)>;
constexpr size_t kHeader = 5 + 12 + 16;
constexpr size_t kLimit = 65536;

// False means no fallback record. Empty authenticated plaintext is a sign-out
// tombstone: a returning Secret Service must not resurrect its older bundle.
gchar* ReadFile(bool& present, GError** error) {
  try {
    File dir(Directory());
    const int fd = openat(dir.fd, "session", O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
    if (fd < 0 && errno == ENOENT) { present = false; return nullptr; }
    present = true;
    File file(fd);
    auto bytes = ReadAll(file.fd, kHeader + kLimit);
    Check(bytes.size() >= kHeader && std::memcmp(bytes.data(), "AFUG1", 5) == 0);
    Key key;
    LoadKey(dir.fd, key, false);
    Cipher ctx(EVP_CIPHER_CTX_new(), EVP_CIPHER_CTX_free);
    Check(ctx != nullptr);
    Check(EVP_DecryptInit_ex(ctx.get(), EVP_aes_256_gcm(), nullptr, key.bytes.data(), bytes.data() + 5) == 1);
    int n = 0;
    Check(EVP_DecryptUpdate(ctx.get(), nullptr, &n, bytes.data(), 5) == 1);
    Plaintext plain;
    plain.bytes.resize(bytes.size() - kHeader + 1);
    Check(EVP_DecryptUpdate(ctx.get(), plain.bytes.data(), &n, bytes.data() + kHeader,
                           static_cast<int>(bytes.size() - kHeader)) == 1);
    int length = n;
    Check(EVP_CIPHER_CTX_ctrl(ctx.get(), EVP_CTRL_GCM_SET_TAG, 16, bytes.data() + 17) == 1);
    Check(EVP_DecryptFinal_ex(ctx.get(), plain.bytes.data() + length, &n) == 1);
    length += n;
    Check(length >= 0 && static_cast<size_t>(length) < plain.bytes.size());
    plain.bytes[length] = 0;
    Check(std::strlen(reinterpret_cast<const char*>(plain.bytes.data())) == static_cast<size_t>(length));
    return length == 0 ? nullptr : g_strdup(reinterpret_cast<const char*>(plain.bytes.data()));
  } catch (...) { present = true; Error(error); return nullptr; }
}
bool WriteFile(const gchar* value, GError** error) {
  try {
    const size_t length = std::strlen(value);
    Check(length <= kLimit);
    File dir(Directory());
    Key key;
    LoadKey(dir.fd, key, true);
    std::vector<unsigned char> bytes(kHeader + length + 16);
    std::memcpy(bytes.data(), "AFUG1", 5);
    Check(RAND_bytes(bytes.data() + 5, 12) == 1);
    Cipher ctx(EVP_CIPHER_CTX_new(), EVP_CIPHER_CTX_free);
    Check(ctx != nullptr);
    Check(EVP_EncryptInit_ex(ctx.get(), EVP_aes_256_gcm(), nullptr, key.bytes.data(), bytes.data() + 5) == 1);
    int n = 0;
    Check(EVP_EncryptUpdate(ctx.get(), nullptr, &n, bytes.data(), 5) == 1);
    Check(EVP_EncryptUpdate(ctx.get(), bytes.data() + kHeader, &n,
                           reinterpret_cast<const unsigned char*>(value), static_cast<int>(length)) == 1);
    int encrypted = n;
    Check(EVP_EncryptFinal_ex(ctx.get(), bytes.data() + kHeader + encrypted, &n) == 1);
    encrypted += n;
    Check(EVP_CIPHER_CTX_ctrl(ctx.get(), EVP_CTRL_GCM_GET_TAG, 16, bytes.data() + 17) == 1);
    bytes.resize(kHeader + encrypted);
    std::array<unsigned char, 8> random {};
    Check(RAND_bytes(random.data(), random.size()) == 1);
    gchar* encoded = g_base64_encode(random.data(), random.size());
    std::string temporary = std::string("session-") + encoded;
    g_free(encoded);
    for (auto& c : temporary) if (c == '/' || c == '+') c = '_';
    File file(openat(dir.fd, temporary.c_str(), O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, 0600));
    try {
      PrivateFile(file.fd);
      WriteAll(file.fd, bytes.data(), bytes.size());
      Check(renameat(dir.fd, temporary.c_str(), dir.fd, "session") == 0);
      Check(fsync(dir.fd) == 0);
    } catch (...) { unlinkat(dir.fd, temporary.c_str(), 0); throw; }
    return true;
  } catch (...) { Error(error); return false; }
}
}
namespace afudesk {
gchar* ReadGoogleSession(GError** error) {
  try {
  StoreLock lock;
  bool present = false;
  gchar* fallback = ReadFile(present, error);
  if (present) return fallback;
  gchar* value = secret_password_lookup_sync(&kSessionSchema, nullptr, error,
                                           "slot", "google-session", nullptr);
  if (error != nullptr && Unavailable(*error)) {
    g_clear_error(error);
    return nullptr;
  }
  return value;
  } catch (...) { Error(error); return nullptr; }
}
gboolean WriteGoogleSession(const gchar* value, GError** error) {
  try {
  StoreLock lock;
  if (value == nullptr || std::strlen(value) == 0 || std::strlen(value) > kLimit) {
    Error(error); return false;
  }
  // Once fallback was selected, keep using that backend. A reappearing keyring
  // cannot resurrect an old refresh token or create a partial migration.
  if (HasFile(lock.directory.fd)) return WriteFile(value, error);
  const bool saved = secret_password_store_sync(&kSessionSchema, SECRET_COLLECTION_DEFAULT,
      "AfuDesk Google session", value, nullptr, error,
      "slot", "google-session", nullptr);
  if (saved) return true;
  if (error != nullptr && Unavailable(*error)) {
    g_clear_error(error);
    return WriteFile(value, error);
  }
  return false;
  } catch (...) { Error(error); return false; }
}
gboolean ClearGoogleSession(GError** error) {
  try {
  StoreLock lock;
  const bool fallback = HasFile(lock.directory.fd);
  // A sticky fallback keeps an authenticated sign-out marker; a returning
  // keyring must never expose a stale session left in the other backend.
  if (fallback && !WriteFile("", error)) return false;
  secret_password_clear_sync(&kSessionSchema, nullptr, error,
                            "slot", "google-session", nullptr);
  if (error != nullptr && Unavailable(*error)) { g_clear_error(error); return fallback || WriteFile("", error); }
  if (error != nullptr && *error != nullptr) return false;
  return true;
  } catch (...) { Error(error); return false; }
}
}
