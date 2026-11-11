#!/usr/bin/env bash
set -euo pipefail
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$repo/app"
flutter config --enable-linux-desktop
flutter pub get
flutter analyze
flutter test
# This override applies only to the Linux binary; pubspec/Windows stay intact.
flutter build linux --release --build-name=0.1.0 --build-number=1 \
  --dart-define="AFUDESK_GOOGLE_DESKTOP_CLIENT_ID=${AFUDESK_GOOGLE_DESKTOP_CLIENT_ID:-}"
