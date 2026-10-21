#include "flutter_window.h"

#include <optional>
#include <shellapi.h>
#include <stdexcept>
#include <vector>
#include <flutter/method_channel.h>
#include <flutter/standard_method_codec.h>

#include "flutter/generated_plugin_registrant.h"
#include "google_secure_storage.h"
#include "resource.h"

namespace {
constexpr UINT kTray = WM_APP + 17;
constexpr UINT kOpen = 1001, kStop = 1002, kExit = 1003;
constexpr wchar_t kRun[] = L"Software\\Microsoft\\Windows\\CurrentVersion\\Run";

std::wstring StartupCommand() {
  std::vector<wchar_t> path(32768);
  const DWORD size = GetModuleFileNameW(nullptr, path.data(), static_cast<DWORD>(path.size()));
  if (!size || size >= path.size()) throw std::runtime_error("startup");
  return L"\"" + std::wstring(path.data(), size) + L"\" --arka-plan";
}

bool StartupEnabled() {
  DWORD type = 0, size = 0;
  const LSTATUS status = RegGetValueW(HKEY_CURRENT_USER, kRun, L"AfuDesk",
      RRF_RT_REG_SZ, &type, nullptr, &size);
  if (status == ERROR_FILE_NOT_FOUND) return false;
  if (status != ERROR_SUCCESS) throw std::runtime_error("startup");
  return size > sizeof(wchar_t);
}

void SetStartup(bool enabled) {
  // Resolve before opening the key so failure cannot leak a registry handle.
  const auto command = enabled ? StartupCommand() : std::wstring();
  HKEY key = nullptr;
  if (RegCreateKeyExW(HKEY_CURRENT_USER, kRun, 0, nullptr, 0,
                     KEY_SET_VALUE, nullptr, &key, nullptr) != ERROR_SUCCESS)
    throw std::runtime_error("startup");
  LSTATUS status;
  if (enabled) {
    status = RegSetValueExW(key, L"AfuDesk", 0, REG_SZ,
        reinterpret_cast<const BYTE*>(command.c_str()),
        static_cast<DWORD>((command.size() + 1) * sizeof(wchar_t)));
  } else {
    status = RegDeleteValueW(key, L"AfuDesk");
  }
  RegCloseKey(key);
  if (status != ERROR_SUCCESS && !(status == ERROR_FILE_NOT_FOUND && !enabled))
    throw std::runtime_error("startup");
}
}

FlutterWindow::FlutterWindow(const flutter::DartProject& project, bool background)
    : project_(project), background_(background), start_hidden_(background) {}

FlutterWindow::~FlutterWindow() {}

bool FlutterWindow::OnCreate() {
  if (!Win32Window::OnCreate()) {
    return false;
  }

  RECT frame = GetClientArea();

  // The size here must match the window dimensions to avoid unnecessary surface
  // creation / destruction in the startup path.
  flutter_controller_ = std::make_unique<flutter::FlutterViewController>(
      frame.right - frame.left, frame.bottom - frame.top, project_);
  // Ensure that basic setup of the controller was successful.
  if (!flutter_controller_->engine() || !flutter_controller_->view()) {
    return false;
  }
  RegisterPlugins(flutter_controller_->engine());
  // Refresh an existing opt-in after an installer moved the executable.
  try {
    if (StartupEnabled()) {
      SetStartup(true);
      background_ = true;
    }
  } catch (...) { /* Settings can report/retry a registry failure. */ }
  if (background_ && !AddTray()) background_ = false;
  background_channel_ = std::make_unique<flutter::MethodChannel<flutter::EncodableValue>>(
      flutter_controller_->engine()->messenger(), "afudesk/arka_plan",
      &flutter::StandardMethodCodec::GetInstance());
  background_channel_->SetMethodCallHandler([this](const auto& call, auto result) {
    try {
      if (call.method_name() == "oku") {
        result->Success(flutter::EncodableValue(StartupEnabled()));
      } else if (call.method_name() == "kapat") {
        quitting_ = true;
        result->Success();
        PostMessageW(GetHandle(), WM_CLOSE, 0, 0);
      } else if (call.method_name() == "ayarla") {
        const bool enabled = call.arguments() && std::holds_alternative<std::string>(*call.arguments());
        // Construct the command from the native executable, never Dart input.
        const bool had_tray = tray_added_;
        if (enabled && !AddTray()) throw std::runtime_error("tray");
        try { SetStartup(enabled); }
        catch (...) {
          if (!had_tray) RemoveTray();
          throw;
        }
        background_ = enabled;
        if (!enabled) { Show(); RemoveTray(); }
        result->Success();
      } else result->NotImplemented();
    } catch (...) {
      result->Error("background", "Başlangıç ayarı değiştirilemedi, yeniden dene.");
    }
  });
  secure_channel_ = std::make_unique<flutter::MethodChannel<flutter::EncodableValue>>(
      flutter_controller_->engine()->messenger(), "afudesk/secure_storage",
      &flutter::StandardMethodCodec::GetInstance());
  secure_channel_->SetMethodCallHandler([](const auto& call, auto result) {
    try {
      const auto path = afudesk::GoogleSessionPath();
      if (call.method_name() == "write") {
        const auto* value = call.arguments() ? std::get_if<std::string>(call.arguments()) : nullptr;
        if (!value) {
          result->Error("secure_storage", "Oturum kaydedilemedi; yeniden giriş yap.");
          return;
        }
        afudesk::WriteGoogleSession(path, *value);
        result->Success();
      } else if (call.method_name() == "read") {
        const auto value = afudesk::ReadGoogleSession(path);
        if (value) result->Success(flutter::EncodableValue(*value));
        else result->Success();
      } else if (call.method_name() == "clear") {
        afudesk::ClearGoogleSession(path);
        result->Success();
      } else result->NotImplemented();
    } catch (const std::exception&) {
      result->Error("secure_storage", "Oturum kaydı açılamadı; yeniden giriş yap.");
    }
  });
  flutter::MethodChannel<flutter::EncodableValue> update_channel(
      flutter_controller_->engine()->messenger(), "afudesk/guncelle",
      &flutter::StandardMethodCodec::GetInstance());
  update_channel.SetMethodCallHandler([](const auto& call, auto result) {
    if (call.method_name() == "surum") {
      result->Success(flutter::EncodableValue(FLUTTER_VERSION));
    } else {
      result->NotImplemented();
    }
  });
  SetChildContent(flutter_controller_->view()->GetNativeWindow());

  flutter_controller_->engine()->SetNextFrameCallback([&]() {
    // Only --arka-plan suppresses the initial window; ordinary launches open it.
    if (!background_ || !start_hidden_) this->Show();
  });

  // Flutter can complete the first frame before the "show window" callback is
  // registered. The following call ensures a frame is pending to ensure the
  // window is shown. It is a no-op if the first frame hasn't completed yet.
  flutter_controller_->ForceRedraw();

  return true;
}

void FlutterWindow::OnDestroy() {
  RemoveTray();
  if (background_channel_) {
    background_channel_->SetMethodCallHandler(nullptr);
    background_channel_.reset();
  }
  if (secure_channel_) {
    secure_channel_->SetMethodCallHandler(nullptr);
    secure_channel_.reset();
  }
  if (flutter_controller_) {
    flutter_controller_ = nullptr;
  }

  Win32Window::OnDestroy();
}

LRESULT
FlutterWindow::MessageHandler(HWND hwnd, UINT const message,
                              WPARAM const wparam,
                              LPARAM const lparam) noexcept {
  if (message == WM_APP + 18) { Show(); SetForegroundWindow(hwnd); return 0; }
  if (message == WM_CLOSE && background_ && tray_added_ && !quitting_) {
    ShowWindow(hwnd, SW_HIDE);
    return 0;
  }
  if (message == RegisterWindowMessageW(L"TaskbarCreated")) {
    tray_added_ = false;
    if (background_ && !AddTray()) Show();
    return 0;
  }
  if (message == kTray) {
    if (lparam == WM_LBUTTONDBLCLK) { Show(); SetForegroundWindow(hwnd); }
    if (lparam == WM_RBUTTONUP || lparam == WM_CONTEXTMENU) {
      HMENU menu = CreatePopupMenu();
      AppendMenuW(menu, MF_STRING, kOpen, L"Aç");
      AppendMenuW(menu, MF_STRING, kStop, L"Bağlantıları durdur");
      AppendMenuW(menu, MF_STRING, kExit, L"Çıkış");
      POINT point; GetCursorPos(&point);
      SetForegroundWindow(hwnd);
      const UINT selected = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_RIGHTBUTTON,
          point.x, point.y, 0, hwnd, nullptr);
      DestroyMenu(menu);
      if (selected == kOpen) Show();
      if (selected == kStop && background_channel_)
        background_channel_->InvokeMethod("durdur", nullptr);
      if (selected == kExit) {
        // Dart revokes live access before acknowledging exit; no token deletion.
        if (background_channel_) background_channel_->InvokeMethod("cik", nullptr);
      }
      PostMessage(hwnd, WM_NULL, 0, 0);
    }
    return 0;
  }
  // Give Flutter, including plugins, an opportunity to handle window messages.
  if (flutter_controller_) {
    std::optional<LRESULT> result =
        flutter_controller_->HandleTopLevelWindowProc(hwnd, message, wparam,
                                                      lparam);
    if (result) {
      return *result;
    }
  }

  switch (message) {
    case WM_FONTCHANGE:
      flutter_controller_->engine()->ReloadSystemFonts();
      break;
  }

  return Win32Window::MessageHandler(hwnd, message, wparam, lparam);
}

bool FlutterWindow::AddTray() {
  if (tray_added_) return true;
  NOTIFYICONDATAW data{};
  data.cbSize = sizeof(data); data.hWnd = GetHandle(); data.uID = 1;
  data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
  data.uCallbackMessage = kTray;
  data.hIcon = LoadIcon(GetModuleHandle(nullptr), MAKEINTRESOURCE(IDI_APP_ICON));
  wcscpy_s(data.szTip, L"AfuDesk");
  tray_added_ = Shell_NotifyIconW(NIM_ADD, &data) != FALSE;
  return tray_added_;
}

void FlutterWindow::RemoveTray() {
  if (!tray_added_) return;
  NOTIFYICONDATAW data{};
  data.cbSize = sizeof(data); data.hWnd = GetHandle(); data.uID = 1;
  Shell_NotifyIconW(NIM_DELETE, &data);
  tray_added_ = false;
}
