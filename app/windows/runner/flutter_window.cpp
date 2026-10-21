#include "flutter_window.h"

#include <optional>
#include <flutter/method_channel.h>
#include <flutter/standard_method_codec.h>

#include "flutter/generated_plugin_registrant.h"
#include "google_secure_storage.h"

FlutterWindow::FlutterWindow(const flutter::DartProject& project)
    : project_(project) {}

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
    this->Show();
  });

  // Flutter can complete the first frame before the "show window" callback is
  // registered. The following call ensures a frame is pending to ensure the
  // window is shown. It is a no-op if the first frame hasn't completed yet.
  flutter_controller_->ForceRedraw();

  return true;
}

void FlutterWindow::OnDestroy() {
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
