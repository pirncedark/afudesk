#include "dxgi_fix.h"

#include <windows.h>
#include <dxgi1_2.h>
#include <stdio.h>

#include <atomic>
#include <cctype>
#include <string>

#include "startup_log.h"
#include "utils.h"

namespace afudesk {
namespace {

bool ContainsCaseInsensitive(const std::string& haystack, const char* needle) {
  if (!needle || *needle == '\0') {
    return false;
  }
  std::string h_lower;
  h_lower.reserve(haystack.size());
  for (unsigned char c : haystack) {
    h_lower.push_back(static_cast<char>(std::tolower(c)));
  }
  std::string n_lower;
  for (const char* p = needle; *p != '\0'; ++p) {
    n_lower.push_back(
        static_cast<char>(std::tolower(static_cast<unsigned char>(*p))));
  }
  return h_lower.find(n_lower) != std::string::npos;
}

bool IsFlipCompatibleFormat(DXGI_FORMAT fmt) {
  switch (fmt) {
    case DXGI_FORMAT_B8G8R8A8_UNORM:
    case DXGI_FORMAT_R8G8B8A8_UNORM:
    case DXGI_FORMAT_R16G16B16A16_FLOAT:
    case DXGI_FORMAT_R10G10B10A2_UNORM:
      return true;
    default:
      return false;
  }
}

const char* SwapEffectName(DXGI_SWAP_EFFECT effect) {
  switch (effect) {
    case DXGI_SWAP_EFFECT_DISCARD:
      return "DISCARD";
    case DXGI_SWAP_EFFECT_SEQUENTIAL:
      return "SEQUENTIAL";
    case DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL:
      return "FLIP_SEQUENTIAL";
    case DXGI_SWAP_EFFECT_FLIP_DISCARD:
      return "FLIP_DISCARD";
    default:
      return "OTHER";
  }
}

using CreateSwapChainFn = HRESULT(STDMETHODCALLTYPE*)(
    IDXGIFactory* self,
    IUnknown* pDevice,
    DXGI_SWAP_CHAIN_DESC* pDesc,
    IDXGISwapChain** ppSwapChain);

using CreateSwapChainForHwndFn = HRESULT(STDMETHODCALLTYPE*)(
    IDXGIFactory2* self,
    IUnknown* pDevice,
    HWND hWnd,
    const DXGI_SWAP_CHAIN_DESC1* pDesc,
    const DXGI_SWAP_CHAIN_FULLSCREEN_DESC* pFullscreenDesc,
    IDXGIOutput* pRestrictToOutput,
    IDXGISwapChain1** ppSwapChain);

using PresentFn = HRESULT(STDMETHODCALLTYPE*)(IDXGISwapChain* self,
                                              UINT SyncInterval,
                                              UINT Flags);

using Present1Fn = HRESULT(STDMETHODCALLTYPE*)(
    IDXGISwapChain1* self,
    UINT SyncInterval,
    UINT PresentFlags,
    const DXGI_PRESENT_PARAMETERS* pPresentParameters);

using ResizeBuffersFn = HRESULT(STDMETHODCALLTYPE*)(IDXGISwapChain* self,
                                                    UINT BufferCount,
                                                    UINT Width,
                                                    UINT Height,
                                                    DXGI_FORMAT NewFormat,
                                                    UINT SwapChainFlags);

CreateSwapChainFn g_orig_create_swap_chain = nullptr;
CreateSwapChainForHwndFn g_orig_create_swap_chain_for_hwnd = nullptr;
PresentFn g_orig_present = nullptr;
Present1Fn g_orig_present1 = nullptr;
ResizeBuffersFn g_orig_resize_buffers = nullptr;

std::atomic<bool> g_needs_full_present{false};
std::atomic<int> g_create_log_count{0};
std::atomic<int> g_present_log_count{0};
std::atomic<int> g_present1_log_count{0};
std::atomic<int> g_resize_log_count{0};

bool PatchVtableSlot(void** vtbl, size_t index, void* hook, void** out_orig) {
  if (!vtbl || !hook) {
    return false;
  }
  if (vtbl[index] == hook) {
    return true;
  }
  DWORD old_protect = 0;
  if (!VirtualProtect(&vtbl[index], sizeof(void*), PAGE_READWRITE,
                      &old_protect)) {
    return false;
  }
  if (out_orig && *out_orig == nullptr) {
    *out_orig = vtbl[index];
  }
  vtbl[index] = hook;
  DWORD unused = 0;
  VirtualProtect(&vtbl[index], sizeof(void*), old_protect, &unused);
  return true;
}

HRESULT STDMETHODCALLTYPE HookPresent(IDXGISwapChain* self,
                                      UINT SyncInterval,
                                      UINT Flags) {
  g_needs_full_present.store(false);
  const HRESULT hr = g_orig_present(self, SyncInterval, Flags);
  const int n = g_present_log_count.fetch_add(1);
  if (n < 3) {
    char buf[128];
    snprintf(buf, sizeof(buf),
             "dxgi_present call=%d sync=%u flags=0x%x hr=0x%08lx", n,
             SyncInterval, Flags, static_cast<unsigned long>(hr));
    StartupLog(buf);
  }
  return hr;
}

HRESULT STDMETHODCALLTYPE HookPresent1(
    IDXGISwapChain1* self,
    UINT SyncInterval,
    UINT PresentFlags,
    const DXGI_PRESENT_PARAMETERS* pPresentParameters) {
  const bool need_full = g_needs_full_present.exchange(false);
  const DXGI_PRESENT_PARAMETERS* params = pPresentParameters;
  DXGI_PRESENT_PARAMETERS empty_params{};
  bool stripped_dirty = false;
  if (need_full && pPresentParameters &&
      pPresentParameters->DirtyRectsCount > 0) {
    params = &empty_params;
    stripped_dirty = true;
  }
  const HRESULT hr =
      g_orig_present1(self, SyncInterval, PresentFlags, params);
  const int n = g_present1_log_count.fetch_add(1);
  if (n < 4) {
    char buf[160];
    snprintf(buf, sizeof(buf),
             "dxgi_present1 call=%d sync=%u flags=0x%x stripped_dirty=%d hr=0x%08lx",
             n, SyncInterval, PresentFlags, stripped_dirty ? 1 : 0,
             static_cast<unsigned long>(hr));
    StartupLog(buf);
  }
  return hr;
}

HRESULT STDMETHODCALLTYPE HookResizeBuffers(IDXGISwapChain* self,
                                            UINT BufferCount,
                                            UINT Width,
                                            UINT Height,
                                            DXGI_FORMAT NewFormat,
                                            UINT SwapChainFlags) {
  const UINT fixed_count = (BufferCount == 1) ? 0 : BufferCount;
  const UINT fixed_flags = SwapChainFlags & ~DXGI_SWAP_CHAIN_FLAG_GDI_COMPATIBLE;
  const HRESULT hr = g_orig_resize_buffers(self, fixed_count, Width, Height,
                                           NewFormat, fixed_flags);
  if (SUCCEEDED(hr)) {
    g_needs_full_present.store(true);
  }
  const int n = g_resize_log_count.fetch_add(1);
  if (n < 5) {
    char buf[192];
    snprintf(buf, sizeof(buf),
             "dxgi_resize_buffers call=%d %ux%u req_bufs=%u used_bufs=%u fmt=%d hr=0x%08lx",
             n, Width, Height, BufferCount, fixed_count,
             static_cast<int>(NewFormat), static_cast<unsigned long>(hr));
    StartupLog(buf);
  }
  return hr;
}

void InstallSwapChainHooks(IDXGISwapChain* sc) {
  if (!sc) {
    return;
  }
  void** vtbl = *reinterpret_cast<void***>(sc);
  const bool p_ok = PatchVtableSlot(
      vtbl, 8, reinterpret_cast<void*>(&HookPresent),
      reinterpret_cast<void**>(&g_orig_present));
  const bool r_ok = PatchVtableSlot(
      vtbl, 13, reinterpret_cast<void*>(&HookResizeBuffers),
      reinterpret_cast<void**>(&g_orig_resize_buffers));

  bool p1_ok = false;
  IDXGISwapChain1* sc1 = nullptr;
  if (SUCCEEDED(sc->QueryInterface(__uuidof(IDXGISwapChain1),
                                   reinterpret_cast<void**>(&sc1))) &&
      sc1) {
    void** vtbl1 = *reinterpret_cast<void***>(sc1);
    p1_ok = PatchVtableSlot(vtbl1, 22, reinterpret_cast<void*>(&HookPresent1),
                            reinterpret_cast<void**>(&g_orig_present1));
    sc1->Release();
  }

  static std::atomic<bool> logged_once{false};
  if (!logged_once.exchange(true)) {
    char buf[128];
    snprintf(buf, sizeof(buf),
             "dxgi_swapchain_hooks present=%d present1=%d resize=%d",
             p_ok ? 1 : 0, p1_ok ? 1 : 0, r_ok ? 1 : 0);
    StartupLog(buf);
  }
}

HRESULT STDMETHODCALLTYPE HookCreateSwapChainForHwnd(
    IDXGIFactory2* self,
    IUnknown* pDevice,
    HWND hWnd,
    const DXGI_SWAP_CHAIN_DESC1* pDesc,
    const DXGI_SWAP_CHAIN_FULLSCREEN_DESC* pFullscreenDesc,
    IDXGIOutput* pRestrictToOutput,
    IDXGISwapChain1** ppSwapChain) {
  const int log_idx = g_create_log_count.fetch_add(1);
  if (pDesc && log_idx < 8) {
    char buf[256];
    snprintf(
        buf, sizeof(buf),
        "dxgi_create_sc_hwnd req#%d hwnd=0x%llx %ux%u fmt=%d bufs=%u effect=%s(%d) scaling=%d samples=%u flags=0x%x",
        log_idx,
        static_cast<unsigned long long>(reinterpret_cast<uintptr_t>(hWnd)),
        pDesc->Width, pDesc->Height, static_cast<int>(pDesc->Format),
        pDesc->BufferCount, SwapEffectName(pDesc->SwapEffect),
        static_cast<int>(pDesc->SwapEffect), static_cast<int>(pDesc->Scaling),
        pDesc->SampleDesc.Count, pDesc->Flags);
    StartupLog(buf);
  }

  const bool should_upgrade =
      pDesc != nullptr &&
      (pDesc->SwapEffect == DXGI_SWAP_EFFECT_DISCARD ||
       pDesc->SwapEffect == DXGI_SWAP_EFFECT_SEQUENTIAL) &&
      pDesc->SampleDesc.Count == 1 && IsFlipCompatibleFormat(pDesc->Format);

  if (should_upgrade) {
    DXGI_SWAP_CHAIN_DESC1 upgraded = *pDesc;
    upgraded.SwapEffect = DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL;
    if (upgraded.BufferCount < 2) {
      upgraded.BufferCount = 2;
    }
    upgraded.Flags &= ~DXGI_SWAP_CHAIN_FLAG_GDI_COMPATIBLE;
    upgraded.Stereo = FALSE;
    upgraded.AlphaMode = DXGI_ALPHA_MODE_UNSPECIFIED;

    const HRESULT hr = g_orig_create_swap_chain_for_hwnd(
        self, pDevice, hWnd, &upgraded, pFullscreenDesc, pRestrictToOutput,
        ppSwapChain);
    if (SUCCEEDED(hr) && ppSwapChain && *ppSwapChain) {
      g_needs_full_present.store(true);
      char buf[160];
      snprintf(buf, sizeof(buf),
               "dxgi_swapchain_hwnd_upgraded swap_effect=FLIP_SEQUENTIAL bufs=%u hr=0x%08lx",
               upgraded.BufferCount, static_cast<unsigned long>(hr));
      StartupLog(buf);
      InstallSwapChainHooks(*ppSwapChain);
      return hr;
    }
    char buf[128];
    snprintf(buf, sizeof(buf),
             "dxgi_swapchain_hwnd_upgrade_failed hr=0x%08lx fallback=orig",
             static_cast<unsigned long>(hr));
    StartupLog(buf);
  }

  const HRESULT hr = g_orig_create_swap_chain_for_hwnd(
      self, pDevice, hWnd, pDesc, pFullscreenDesc, pRestrictToOutput,
      ppSwapChain);
  if (SUCCEEDED(hr) && ppSwapChain && *ppSwapChain) {
    InstallSwapChainHooks(*ppSwapChain);
  }
  return hr;
}

HRESULT STDMETHODCALLTYPE HookCreateSwapChain(
    IDXGIFactory* self,
    IUnknown* pDevice,
    DXGI_SWAP_CHAIN_DESC* pDesc,
    IDXGISwapChain** ppSwapChain) {
  const int log_idx = g_create_log_count.fetch_add(1);
  if (pDesc && log_idx < 8) {
    char buf[256];
    snprintf(
        buf, sizeof(buf),
        "dxgi_create_sc req#%d hwnd=0x%llx %ux%u fmt=%d bufs=%u windowed=%d effect=%s(%d) samples=%u flags=0x%x",
        log_idx,
        static_cast<unsigned long long>(
            reinterpret_cast<uintptr_t>(pDesc->OutputWindow)),
        pDesc->BufferDesc.Width, pDesc->BufferDesc.Height,
        static_cast<int>(pDesc->BufferDesc.Format), pDesc->BufferCount,
        pDesc->Windowed ? 1 : 0, SwapEffectName(pDesc->SwapEffect),
        static_cast<int>(pDesc->SwapEffect), pDesc->SampleDesc.Count,
        pDesc->Flags);
    StartupLog(buf);
  }

  const bool should_upgrade =
      pDesc != nullptr && pDesc->Windowed &&
      (pDesc->SwapEffect == DXGI_SWAP_EFFECT_DISCARD ||
       pDesc->SwapEffect == DXGI_SWAP_EFFECT_SEQUENTIAL) &&
      pDesc->SampleDesc.Count == 1 &&
      IsFlipCompatibleFormat(pDesc->BufferDesc.Format);

  if (should_upgrade) {
    DXGI_SWAP_CHAIN_DESC upgraded = *pDesc;
    upgraded.SwapEffect = DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL;
    if (upgraded.BufferCount < 2) {
      upgraded.BufferCount = 2;
    }
    upgraded.Flags &= ~DXGI_SWAP_CHAIN_FLAG_GDI_COMPATIBLE;

    const HRESULT hr =
        g_orig_create_swap_chain(self, pDevice, &upgraded, ppSwapChain);
    if (SUCCEEDED(hr) && ppSwapChain && *ppSwapChain) {
      g_needs_full_present.store(true);
      char buf[160];
      snprintf(buf, sizeof(buf),
               "dxgi_swapchain_upgraded swap_effect=FLIP_SEQUENTIAL bufs=%u hr=0x%08lx",
               upgraded.BufferCount, static_cast<unsigned long>(hr));
      StartupLog(buf);
      InstallSwapChainHooks(*ppSwapChain);
      return hr;
    }
    char buf[128];
    snprintf(buf, sizeof(buf),
             "dxgi_swapchain_upgrade_failed hr=0x%08lx fallback=orig",
             static_cast<unsigned long>(hr));
    StartupLog(buf);
  }

  const HRESULT hr = g_orig_create_swap_chain(self, pDevice, pDesc, ppSwapChain);
  if (SUCCEEDED(hr) && ppSwapChain && *ppSwapChain) {
    InstallSwapChainHooks(*ppSwapChain);
  }
  return hr;
}

}  // namespace

void LogDxgiAdapters() {
  IDXGIFactory1* factory = nullptr;
  const HRESULT fhr = CreateDXGIFactory1(
      __uuidof(IDXGIFactory1), reinterpret_cast<void**>(&factory));
  if (FAILED(fhr) || !factory) {
    char buf[96];
    snprintf(buf, sizeof(buf), "dxgi_enum_failed hr=0x%08lx",
             static_cast<unsigned long>(fhr));
    StartupLog(buf);
    return;
  }

  for (UINT i = 0;; ++i) {
    IDXGIAdapter1* adapter = nullptr;
    if (factory->EnumAdapters1(i, &adapter) == DXGI_ERROR_NOT_FOUND ||
        !adapter) {
      break;
    }
    DXGI_ADAPTER_DESC1 desc{};
    if (SUCCEEDED(adapter->GetDesc1(&desc))) {
      UINT outputs = 0;
      for (UINT j = 0;; ++j) {
        IDXGIOutput* out = nullptr;
        if (adapter->EnumOutputs(j, &out) == DXGI_ERROR_NOT_FOUND || !out) {
          break;
        }
        ++outputs;
        out->Release();
      }
      const std::string name = Utf8FromUtf16(desc.Description);
      const bool is_sudomaker = ContainsCaseInsensitive(name, "sudomaker") ||
                                ContainsCaseInsensitive(name, "sudovda");
      const bool is_basic_render =
          (desc.VendorId == 0x1414 && desc.DeviceId == 0x008c) ||
          ((desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE) != 0);
      const unsigned long long vram_mb =
          static_cast<unsigned long long>(desc.DedicatedVideoMemory) /
          (1024ULL * 1024ULL);
      char buf[384];
      snprintf(
          buf, sizeof(buf),
          "dxgi_adapter[%u]=%s vendor=0x%04x device=0x%04x vram_mb=%llu luid=%ld:%lu flags=0x%x outputs=%u sudomaker=%d basic_render=%d",
          i, name.c_str(), desc.VendorId, desc.DeviceId, vram_mb,
          desc.AdapterLuid.HighPart, desc.AdapterLuid.LowPart, desc.Flags,
          outputs, is_sudomaker ? 1 : 0, is_basic_render ? 1 : 0);
      StartupLog(buf);
    }
    adapter->Release();
  }
  factory->Release();

  DISPLAY_DEVICEW dd{};
  dd.cb = sizeof(dd);
  int sudomaker_displays = 0;
  int attached_displays = 0;
  for (DWORD i = 0; EnumDisplayDevicesW(nullptr, i, &dd, 0); ++i) {
    const std::string dev_name = Utf8FromUtf16(dd.DeviceName);
    const std::string dev_str = Utf8FromUtf16(dd.DeviceString);
    const bool attached = (dd.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP) != 0;
    const bool primary = (dd.StateFlags & DISPLAY_DEVICE_PRIMARY_DEVICE) != 0;
    const bool is_sudo = ContainsCaseInsensitive(dev_str, "sudomaker") ||
                         ContainsCaseInsensitive(Utf8FromUtf16(dd.DeviceID),
                                                 "sudovda");
    if (is_sudo) {
      ++sudomaker_displays;
    }
    if (attached) {
      ++attached_displays;
      char buf[256];
      snprintf(buf, sizeof(buf),
               "display_attached=%s driver=%s primary=%d sudomaker=%d flags=0x%lx",
               dev_name.c_str(), dev_str.c_str(), primary ? 1 : 0,
               is_sudo ? 1 : 0, dd.StateFlags);
      StartupLog(buf);
    }
    dd = DISPLAY_DEVICEW{};
    dd.cb = sizeof(dd);
  }
  char summary[128];
  snprintf(summary, sizeof(summary),
           "display_summary attached=%d sudomaker_entries=%d",
           attached_displays, sudomaker_displays);
  StartupLog(summary);
}

void InstallDxgiFlipSwapChainFix() {
  static bool installed = false;
  if (installed) {
    return;
  }
  installed = true;

  IDXGIFactory2* factory = nullptr;
  const HRESULT hr = CreateDXGIFactory1(
      __uuidof(IDXGIFactory2), reinterpret_cast<void**>(&factory));
  if (FAILED(hr) || !factory) {
    char buf[96];
    snprintf(buf, sizeof(buf), "dxgi_hook_factory_failed hr=0x%08lx",
             static_cast<unsigned long>(hr));
    StartupLog(buf);
    return;
  }

  void** vtbl = *reinterpret_cast<void***>(factory);
  const bool ok_sc = PatchVtableSlot(
      vtbl, 10, reinterpret_cast<void*>(&HookCreateSwapChain),
      reinterpret_cast<void**>(&g_orig_create_swap_chain));
  const bool ok_hwnd = PatchVtableSlot(
      vtbl, 15, reinterpret_cast<void*>(&HookCreateSwapChainForHwnd),
      reinterpret_cast<void**>(&g_orig_create_swap_chain_for_hwnd));
  factory->Release();

  char buf[128];
  snprintf(buf, sizeof(buf), "dxgi_flip_hook_installed sc=%d sc_hwnd=%d",
           ok_sc ? 1 : 0, ok_hwnd ? 1 : 0);
  StartupLog(buf);
}

}  // namespace afudesk
