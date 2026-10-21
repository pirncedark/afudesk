#ifndef RUNNER_GOOGLE_SECURE_STORAGE_H_
#define RUNNER_GOOGLE_SECURE_STORAGE_H_

#include <filesystem>
#include <optional>
#include <string>

namespace afudesk {
// Throws on IO, authentication or encryption failure. No plaintext is persisted.
std::filesystem::path GoogleSessionPath();
void WriteGoogleSession(const std::filesystem::path& path, const std::string& value);
std::optional<std::string> ReadGoogleSession(const std::filesystem::path& path);
void ClearGoogleSession(const std::filesystem::path& path);
}  // namespace afudesk
#endif
