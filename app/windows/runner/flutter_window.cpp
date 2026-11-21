#include "flutter_window.h"

#include <flutter_windows.h>
#include <stdio.h>

#include <optional>
#include <shellapi.h>
#include <stdexcept>
#include <string>
#include <vector>
#include <flutter/method_channel.h>
#include <flutter/standard_method_codec.h>

#include "flutter/generated_plugin_registrant.h"
#include "google_secure_storage.h"
#include "resource.h"
#include "startup_log.h"
#include "utils.h"

namespace {
constexpr UINT kTray = WM_APP + 17;
constexpr UINT kOpen = 1001, kStop = 1002, kExit = 1003;
constexpr UINT_PTR kExitWatchdog = 77;
// Ilk kare geri cagrisi bazen hic gelmez; pencere gorunmez kalir (bkz. acilis.log).
constexpr UINT_PTR kShowFallback = 78;
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
    afudesk::StartupErrorLog("Win32Window::OnCreate returned false");
    return false;
  }

  RECT frame = GetClientArea();
  afudesk::StartupLog(
      "oncreate_begin rect=" + std::to_string(frame.right - frame.left) + "x" +
      std::to_string(frame.bottom - frame.top));

  // The size here must match the window dimensions to avoid unnecessary surface
  // creation / destruction in the startup path.
  afudesk::StartupLog("flutter_controller_create_begin");
  flutter_controller_ = std::make_unique<flutter::FlutterViewController>(
      frame.right - frame.left, frame.bottom - frame.top, project_);
  const bool has_engine = flutter_controller_->engine() != nullptr;
  const bool has_view = flutter_controller_->view() != nullptr;
  afudesk::StartupLog(std::string("flutter_controller_create_done engine=") +
                      (has_engine ? "1" : "0") + " view=" +
                      (has_view ? "1" : "0"));
  // Ensure that basic setup of the controller was successful.
  if (!has_engine || !has_view) {
    afudesk::StartupErrorLog("FlutterViewController engine or view is null");
    return false;
  }

  IDXGIAdapter* adapter = nullptr;
  auto* diag_registrar =
      flutter_controller_->engine()->GetRegistrarForPlugin("AfuDeskDiag");
  if (diag_registrar &&
      FlutterDesktopPluginRegistrarGetGraphicsAdapter(diag_registrar,
                                                    &adapter) &&
      adapter) {
    DXGI_ADAPTER_DESC desc{};
    if (SUCCEEDED(adapter->GetDesc(&desc))) {
      char buf[192];
      const unsigned long long vram_mb =
          static_cast<unsigned long long>(desc.DedicatedVideoMemory) /
          (1024ULL * 1024ULL);
      snprintf(buf, sizeof(buf),
               " vendor=0x%04x device=0x%04x luid=%ld:%lu vram_mb=%llu",
               desc.VendorId, desc.DeviceId, desc.AdapterLuid.HighPart,
               desc.AdapterLuid.LowPart, vram_mb);
      afudesk::StartupLog("gpu_adapter=" + Utf8FromUtf16(desc.Description) +
                          buf);
    } else {
      afudesk::StartupLog("gpu_adapter=get_desc_failed");
    }
    adapter->Release();
  } else {
    afudesk::StartupLog("gpu_adapter=none_or_software");
  }

  RegisterPlugins(flutter_controller_->engine());
  afudesk::StartupLog("register_plugins_done");

  if (GetEnvironmentVariableW(L"AFUDESK_NO_STARTUP_UPDATE", nullptr, 0) > 0) {
    afudesk::StartupLog("startup_refresh_skipped_env");
    if (GetEnvironmentVariableW(L"AFUDESK_TEST_TRAY", nullptr, 0) > 0) {
      background_ = true;
      afudesk::StartupLog("test_tray_enabled");
    }
  } else {
    // Refresh an existing opt-in after an installer moved the executable.
    try {
      if (StartupEnabled()) {
        SetStartup(true);
        background_ = true;
      }
    } catch (...) { /* Settings can report/retry a registry failure. */ }
  }
  if (background_) {
    afudesk::StartupLog("add_tray_begin");
    const bool ok = AddTray();
    afudesk::StartupLog(std::string("add_tray_done ok=") + (ok ? "1" : "0"));
    if (!ok) background_ = false;
  }
  afudesk::StartupLog(std::string("startup_and_tray_done background=") +
                      (background_ ? "1" : "0") + " start_hidden=" +
                      (start_hidden_ ? "1" : "0") + " tray_added=" +
                      (tray_added_ ? "1" : "0"));
  background_channel_ = std::make_unique<flutter::MethodChannel<flutter::EncodableValue>>(
      flutter_controller_->engine()->messenger(), "afudesk/arka_plan",
      &flutter::StandardMethodCodec::GetInstance());
  background_channel_->SetMethodCallHandler([this](const auto& call, auto result) {
    try {
      if (call.method_name() == "oku") {
        result->Success(flutter::EncodableValue(StartupEnabled()));
      } else if (call.method_name() == "kapat") {
        quitting_ = true;
        SetTimer(GetHandle(), kExitWatchdog, 3000, nullptr);
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
  update_channel_ = std::make_unique<flutter::MethodChannel<flutter::EncodableValue>>(
      flutter_controller_->engine()->messenger(), "afudesk/guncelle",
      &flutter::StandardMethodCodec::GetInstance());
  update_channel_->SetMethodCallHandler([](const auto& call, auto result) {
    if (call.method_name() == "surum") {
      result->Success(flutter::EncodableValue(FLUTTER_VERSION));
    } else {
      result->NotImplemented();
    }
  });
  afudesk::StartupLog("channels_ready");
  HWND child_hwnd = flutter_controller_->view()->GetNativeWindow();
  SetChildContent(child_hwnd);
  {
    char buf[96];
    snprintf(buf, sizeof(buf), "set_child_content_done hwnd=0x%llx child=0x%llx",
             static_cast<unsigned long long>(reinterpret_cast<uintptr_t>(GetHandle())),
             static_cast<unsigned long long>(reinterpret_cast<uintptr_t>(child_hwnd)));
    afudesk::StartupLog(buf);
  }

  flutter_controller_->engine()->SetNextFrameCallback([this]() {
    first_frame_received_ = true;
    afudesk::StartupLog("first_frame_callback_fired");
    // Only --arka-plan suppresses the initial window; ordinary launches open it.
    if (!background_ || !start_hidden_) {
      this->Show();
      InvalidateRect(GetHandle(), nullptr, TRUE);
      UpdateWindow(GetHandle());
      afudesk::StartupLog(std::string("show_from_first_frame visible=") +
                          (IsWindowVisible(GetHandle()) ? "1" : "0"));
    }
  });

  // Flutter can complete the first frame before the "show window" callback is
  // registered. The following call ensures a frame is pending to ensure the
  // window is shown. It is a no-op if the first frame hasn't completed yet.
  flutter_controller_->ForceRedraw();
  SetTimer(GetHandle(), kShowFallback, 1500, nullptr);
  afudesk::StartupLog("set_next_frame_callback_and_redraw_done");

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
  if (update_channel_) {
    update_channel_->SetMethodCallHandler(nullptr);
    update_channel_.reset();
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
  if (message == WM_PAINT && !first_paint_logged_) {
    first_paint_logged_ = true;
    afudesk::StartupLog("first_wm_paint");
  }
  if (message == WM_TIMER && wparam == kShowFallback) {
    KillTimer(hwnd, kShowFallback);
    afudesk::StartupLog(std::string("show_fallback_timer_fired first_frame=") +
                        (first_frame_received_ ? "1" : "0") + " visible=" +
                        (IsWindowVisible(hwnd) ? "1" : "0"));
    // Same rule as the first-frame callback: only --arka-plan keeps the window hidden.
    if (!quitting_ && (!background_ || !start_hidden_)) {
      if (!IsWindowVisible(hwnd)) Show();
      if (flutter_controller_) flutter_controller_->ForceRedraw();
      afudesk::StartupLog(std::string("show_fallback_after_show visible=") +
                          (IsWindowVisible(hwnd) ? "1" : "0"));
    }
    return 0;
  }
  if (message == WM_TIMER && wparam == kExitWatchdog) {
    RemoveTray();
    TerminateProcess(GetCurrentProcess(), 0);
    return 0;
  }
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
      AppendMenuW(menu, MF_STRING, kOpen, L"A\u00e7");
      AppendMenuW(menu, MF_STRING, kStop, L"Ba\u011flant\u0131lar\u0131 durdur");
      AppendMenuW(menu, MF_STRING, kExit, L"\u00c7\u0131k\u0131\u015f");
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
        // Dart yanıt vermese de süreç 3 sn içinde biter.
        quitting_ = true;
        SetTimer(hwnd, kExitWatchdog, 3000, nullptr);
        if (background_channel_) background_channel_->InvokeMethod("cik", nullptr);
        else PostMessageW(hwnd, WM_CLOSE, 0, 0);
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
