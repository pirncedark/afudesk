#include <flutter/dart_project.h>
#include <flutter/flutter_view_controller.h>
#include <windows.h>
#include <algorithm>

#include "flutter_window.h"
#include "utils.h"

int APIENTRY wWinMain(_In_ HINSTANCE instance, _In_opt_ HINSTANCE prev,
                      _In_ wchar_t *command_line, _In_ int show_command) {
  // Attach to console when present (e.g., 'flutter run') or create a
  // new console when running with a debugger.
  if (!::AttachConsole(ATTACH_PARENT_PROCESS) && ::IsDebuggerPresent()) {
    CreateAndAttachConsole();
  }

  // Initialize COM, so that it is available for use in the library and/or
  // plugins.
  ::CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);

  flutter::DartProject project(L"data");

  std::vector<std::string> command_line_arguments =
      GetCommandLineArguments();

  const bool background = std::find(command_line_arguments.begin(),
      command_line_arguments.end(), "--arka-plan") != command_line_arguments.end();
  // Per-user/session singleton. No host or Flutter engine in the second process.
  HANDLE singleton = CreateMutexW(nullptr, FALSE, L"Local\\AfuDesk.BackgroundHost.v1");
  if (!singleton) return EXIT_FAILURE;
  if (GetLastError() == ERROR_ALREADY_EXISTS) {
    if (!background) {
      HWND existing = FindWindowW(L"FLUTTER_RUNNER_WIN32_WINDOW", L"AfuDesk");
      if (existing) PostMessageW(existing, WM_APP + 18, 0, 0);
    }
    CloseHandle(singleton);
    ::CoUninitialize();
    return EXIT_SUCCESS;
  }

  project.set_dart_entrypoint_arguments(std::move(command_line_arguments));

  FlutterWindow window(project, background);
  Win32Window::Point origin(10, 10);
  Win32Window::Size size(1100, 760);
  if (!window.Create(L"AfuDesk", origin, size)) {
    return EXIT_FAILURE;
  }
  window.SetQuitOnClose(true);

  ::MSG msg;
  while (::GetMessage(&msg, nullptr, 0, 0)) {
    ::TranslateMessage(&msg);
    ::DispatchMessage(&msg);
  }

  ::CoUninitialize();
  CloseHandle(singleton);
  return EXIT_SUCCESS;
}
