#!/usr/bin/env bash
set -euo pipefail
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
out=${1:-"$repo/reports/linux"}
binary=${2:-"$repo/app/build/linux/x64/release/bundle/afudesk"}
mkdir -p "$out"
out=$(realpath "$out")
export GDK_BACKEND=x11 LIBGL_ALWAYS_SOFTWARE=1 AFUDESK_ISOLATED_SMOKE=1
unset WAYLAND_DISPLAY
pids=()
cleanup() {
  code=$?
  trap - EXIT
  for pid in "${pids[@]}"; do kill "$pid" 2>/dev/null || true; done
  for pid in "${pids[@]}"; do wait "$pid" 2>/dev/null || true; done
  printf 'smoke_exit=%s\n' "$code" > "$out/status.txt"
  exit "$code"
}
trap cleanup EXIT
openbox > "$out/openbox.log" 2>&1 & pids+=("$!")
for _ in {1..100}; do
  xprop -root _NET_SUPPORTING_WM_CHECK 2>/dev/null | grep -q 'window id' && break
  sleep 0.1
done
xprop -root _NET_SUPPORTING_WM_CHECK | grep -q 'window id'
# The keyring is created only on the disposable CI machine; no real credentials.
printf 'afudesk-ci-disposable\n' | timeout 20s gnome-keyring-daemon --unlock --components=secrets > "$out/keyring.log" 2>&1
timeout 20s "$out/secure-storage-test" > "$out/secure-storage.log" 2>&1
XDG_DATA_HOME="$(mktemp -d)" DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent-afudesk-bus timeout 20s "$out/secure-storage-test" --unavailable >> "$out/secure-storage.log" 2>&1
rm -f -- "$out/input-target.txt" "$out/input.json"
python3 "$repo/scripts/linux/input_receiver.py" "$out" > "$out/receiver.log" 2>&1 & receiver=$!; pids+=("$receiver")
for _ in {1..100}; do
  test -s "$out/input-target.txt" && break
  kill -0 "$receiver"
  sleep 0.1
done
read -r SMOKE_INPUT_X SMOKE_INPUT_Y < "$out/input-target.txt"
export SMOKE_INPUT_X SMOKE_INPUT_Y
receiver_window=$(timeout 10s xdotool search --sync --onlyvisible --name '^AfuDesk smoke input receiver$' | head -1)
timeout 10s xdotool windowactivate --sync "$receiver_window"
timeout 30s "$repo/rust/target/release/examples/linux_smoke" "$out" > "$out/rust-smoke.log" 2>&1
kill "$receiver"
wait "$receiver" || true
"$binary" > "$out/flutter.log" 2>&1 & app_pid=$!; pids+=("$app_pid")
window=''
for _ in {1..150}; do
  kill -0 "$app_pid"
  window=$(xdotool search --onlyvisible --pid "$app_pid" --name '^AfuDesk$' 2>/dev/null | head -1 || true)
  test -n "$window" && break
  sleep 0.1
done
test -n "$window"
xdotool getwindowgeometry --shell "$window" > "$out/flutter-window.txt"
xdotool getwindowname "$window" >> "$out/flutter-window.txt"
sleep 5
kill -0 "$app_pid"
scrot "$out/flutter-desktop.png"
# Ask the window manager to send WM_DELETE_WINDOW, then require a real exit.
timeout 10s xdotool windowactivate --sync "$window"
xdotool key --clearmodifiers alt+F4
for _ in {1..100}; do
  ! kill -0 "$app_pid" 2>/dev/null && break
  sleep 0.1
done
if kill -0 "$app_pid" 2>/dev/null; then
  echo 'FAIL: Flutter did not close within 10 seconds' >&2
  exit 1
fi
wait "$app_pid"
printf 'PASS: Flutter release opened a visible window, stayed alive, and exited cleanly\n' > "$out/flutter-result.txt"
