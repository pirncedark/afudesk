#include <flutter/dart_project.h>
#include <flutter/flutter_view_controller.h>
#include <windows.h>
#include <algorithm>

#include "dxgi_fix.h"
#include "flutter_window.h"
#include "startup_log.h"
#include "utils.h"

namespace {

std::string ReadEnvUtf8(const wchar_t* name) {
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
  return Utf8FromUtf16(value.c_str());
}

}  // namespace

int APIENTRY wWinMain(_In_ HINSTANCE instance, _In_opt_ HINSTANCE prev,
                      _In_ wchar_t *command_line, _In_ int show_command) {
  const bool stdio_redirected = afudesk::RedirectStdioIfRequested();

  // Attach to console when present (e.g., 'flutter run') or create a
  // new console when running with a debugger.
  if (!stdio_redirected && !::AttachConsole(ATTACH_PARENT_PROCESS) &&
      ::IsDebuggerPresent()) {
    CreateAndAttachConsole();
  }

  // Initialize COM, so that it is available for use in the library and/or
  // plugins.
  ::CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);

  flutter::DartProject project(L"data");

  std::string gpu_pref = ReadEnvUtf8(L"AFUDESK_GPU_PREF");
  if (gpu_pref == "low") {
    project.set_gpu_preference(flutter::GpuPreference::LowPowerPreference);
  } else {
    project.set_gpu_preference(flutter::GpuPreference::HighPerformancePreference);
    if (gpu_pref.empty()) {
      gpu_pref = "high(default)";
    }
  }

  const std::string impeller_opt = ReadEnvUtf8(L"AFUDESK_IMPELLER");
  if (impeller_opt == "enabled") {
    project.set_impeller_switch(flutter::ImpellerSwitch::Enabled);
  } else if (impeller_opt == "disabled") {
    project.set_impeller_switch(flutter::ImpellerSwitch::Disabled);
  }

  const std::string ui_thread_opt = ReadEnvUtf8(L"AFUDESK_UI_THREAD");
  if (ui_thread_opt == "platform") {
    project.set_ui_thread_policy(flutter::UIThreadPolicy::RunOnPlatformThread);
  } else if (ui_thread_opt == "separate") {
    project.set_ui_thread_policy(flutter::UIThreadPolicy::RunOnSeparateThread);
  }

  std::vector<std::string> command_line_arguments =
      GetCommandLineArguments();

  const bool background = std::find(command_line_arguments.begin(),
      command_line_arguments.end(), "--arka-plan") != command_line_arguments.end();
  // Per-user/session singleton. No host or Flutter engine in the second process.
  HANDLE singleton = CreateMutexW(nullptr, FALSE, L"Local\\AfuDesk.BackgroundHost.v1");
  if (!singleton) {
    afudesk::StartupErrorLog("CreateMutexW failed err=" +
                             std::to_string(GetLastError()));
    return EXIT_FAILURE;
  }
  if (GetLastError() == ERROR_ALREADY_EXISTS) {
    afudesk::StartupLog(std::string("singleton_already_exists background=") +
                        (background ? "1" : "0"));
    if (!background) {
      HWND existing = FindWindowW(L"FLUTTER_RUNNER_WIN32_WINDOW", L"AfuDesk");
      if (existing) PostMessageW(existing, WM_APP + 18, 0, 0);
    }
    CloseHandle(singleton);
    ::CoUninitialize();
    return EXIT_SUCCESS;
  }

  afudesk::StartupLogReset();
  afudesk::LogDxgiAdapters();
  afudesk::InstallDxgiFlipSwapChainFix();
  const std::string angle_platform = ReadEnvUtf8(L"ANGLE_DEFAULT_PLATFORM");
  afudesk::StartupLog(
      std::string("wWinMain_primary background=") + (background ? "1" : "0") +
      " gpu_pref=" + (gpu_pref.empty() ? "default" : gpu_pref) +
      " impeller=" + (impeller_opt.empty() ? "default" : impeller_opt) +
      " ui_thread=" + (ui_thread_opt.empty() ? "default" : ui_thread_opt) +
      " angle_platform=" + (angle_platform.empty() ? "unset" : angle_platform));

  project.set_dart_entrypoint_arguments(std::move(command_line_arguments));

  FlutterWindow window(project, background);
  Win32Window::Point origin(10, 10);
  Win32Window::Size size(1100, 760);
  afudesk::StartupLog("window_create_begin");
  if (!window.Create(L"AfuDesk", origin, size)) {
    afudesk::StartupLog("window_create_failed");
    afudesk::StartupErrorLog("Win32Window::Create returned false");
    return EXIT_FAILURE;
  }
  afudesk::StartupLog("window_create_ok");
  window.SetQuitOnClose(true);

  afudesk::StartupLog("message_loop_begin");
  ::MSG msg;
  while (::GetMessage(&msg, nullptr, 0, 0)) {
    ::TranslateMessage(&msg);
    ::DispatchMessage(&msg);
  }
  afudesk::StartupLog("message_loop_end");

  ::CoUninitialize();
  CloseHandle(singleton);
  return EXIT_SUCCESS;
}
