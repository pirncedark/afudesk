#ifndef RUNNER_DXGI_FIX_H_
#define RUNNER_DXGI_FIX_H_

namespace afudesk {

// Logs all DXGI adapters and attached Windows display devices to acilis.log.
void LogDxgiAdapters();

// Upgrades legacy BitBlt DXGI swap chains (DISCARD/SEQUENTIAL) to Flip-Model
// (FLIP_SEQUENTIAL) so ANGLE/Flutter presents reliably under DWM + virtual
// display adapters. Safe no-op on fallback if flip-model creation fails.
void InstallDxgiFlipSwapChainFix();

}  // namespace afudesk

#endif  // RUNNER_DXGI_FIX_H_
