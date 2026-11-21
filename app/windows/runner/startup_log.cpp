#include "startup_log.h"

#include <windows.h>
#include <fcntl.h>
#include <io.h>
#include <shlobj.h>
#include <stdio.h>

#include <iostream>
#include <string>

namespace afudesk {
namespace {

std::wstring GetEnvWide(const wchar_t* name) {
  const DWORD needed = GetEnvironmentVariableW(name, nullptr, 0);
  if (needed <= 1) {
    return {};
  }
  std::wstring value(needed, L'\0');
  const DWORD written = GetEnvironmentVariableW(name, value.data(), needed);
  if (written == 0 || written >= needed) {
    return {};
  }
  value.resize(written);
  return value;
}

std::wstring ResolveLogDir() {
  std::wstring dir = GetEnvWide(L"AFUDESK_LOG_DIR");
  if (!dir.empty()) {
    return dir;
  }
  PWSTR local_app_data = nullptr;
  if (SUCCEEDED(SHGetKnownFolderPath(FOLDERID_LocalAppData, 0, nullptr,
                                     &local_app_data)) &&
      local_app_data != nullptr) {
    dir = local_app_data;
    CoTaskMemFree(local_app_data);
    dir += L"\\AfuDesk";
    return dir;
  }
  return {};
}

std::string FormatNow() {
  SYSTEMTIME st{};
  GetLocalTime(&st);
  char buf[40];
  snprintf(buf, sizeof(buf), "%04u-%02u-%02uT%02u:%02u:%02u.%03u",
           static_cast<unsigned>(st.wYear), static_cast<unsigned>(st.wMonth),
           static_cast<unsigned>(st.wDay), static_cast<unsigned>(st.wHour),
           static_cast<unsigned>(st.wMinute), static_cast<unsigned>(st.wSecond),
           static_cast<unsigned>(st.wMilliseconds));
  return std::string(buf);
}

void WriteLogLine(const wchar_t* filename,
                  const std::string& line,
                  bool truncate) {
  const std::wstring dir = ResolveLogDir();
  if (dir.empty()) {
    return;
  }
  SHCreateDirectoryExW(nullptr, dir.c_str(), nullptr);
  const std::wstring path = dir + L"\\" + filename;
  const DWORD access = truncate ? GENERIC_WRITE : FILE_APPEND_DATA;
  const DWORD disp = truncate ? CREATE_ALWAYS : OPEN_ALWAYS;
  HANDLE file = CreateFileW(path.c_str(), access,
                            FILE_SHARE_READ | FILE_SHARE_WRITE, nullptr, disp,
                            FILE_ATTRIBUTE_NORMAL, nullptr);
  if (file == INVALID_HANDLE_VALUE) {
    return;
  }
  const std::string payload = line + "\n";
  DWORD written = 0;
  WriteFile(file, payload.data(), static_cast<DWORD>(payload.size()), &written,
            nullptr);
  FlushFileBuffers(file);
  CloseHandle(file);
}

}  // namespace

void StartupLogReset() {
  const std::string msg =
      FormatNow() + " pid=" + std::to_string(GetCurrentProcessId()) +
      " [win32] start";
  WriteLogLine(L"acilis.log", msg, true);
}

void StartupLog(const std::string& msg) {
  WriteLogLine(L"acilis.log", FormatNow() + " [win32] " + msg, false);
}

void StartupErrorLog(const std::string& msg) {
  WriteLogLine(L"acilis_hata.txt", FormatNow() + " [win32] " + msg, false);
}

bool RedirectStdioIfRequested() {
  const std::wstring log_path = GetEnvWide(L"AFUDESK_STDERR_LOG");
  if (log_path.empty()) {
    return false;
  }
  const size_t slash = log_path.find_last_of(L"\\/");
  if (slash != std::wstring::npos && slash > 0) {
    const std::wstring parent = log_path.substr(0, slash);
    SHCreateDirectoryExW(nullptr, parent.c_str(), nullptr);
  }
  FILE* err_file = nullptr;
  if (_wfreopen_s(&err_file, log_path.c_str(), L"a", stderr) != 0 ||
      err_file == nullptr) {
    return false;
  }
  setvbuf(stderr, nullptr, _IONBF, 0);
  _dup2(_fileno(stderr), _fileno(stdout));
  setvbuf(stdout, nullptr, _IONBF, 0);
  HANDLE os_handle = reinterpret_cast<HANDLE>(_get_osfhandle(_fileno(stderr)));
  if (os_handle != INVALID_HANDLE_VALUE) {
    SetStdHandle(STD_ERROR_HANDLE, os_handle);
    SetStdHandle(STD_OUTPUT_HANDLE, os_handle);
  }
  std::ios::sync_with_stdio();
  return true;
}

}  // namespace afudesk
