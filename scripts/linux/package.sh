#!/usr/bin/env bash
set -euo pipefail
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
bundle=${1:-"$repo/app/build/linux/x64/release/bundle"}
out=${2:-"$repo/reports/linux"}
bundle=$(realpath "$bundle")
mkdir -p "$out"
out=$(realpath "$out")
test -x "$bundle/afudesk"
stage=$(mktemp -d)
trap 'rm -rf -- "$stage"' EXIT
install -d "$stage/DEBIAN" "$stage/opt/afudesk" "$stage/usr/bin" "$stage/usr/share/applications"
cp -a "$bundle/." "$stage/opt/afudesk/"
# dpkg-shlibdeps computes the real distribution-specific sonames (including
# Ubuntu t64 transitions), instead of guessing xcap/GTK runtime dependencies.
mkdir -p "$stage/debian"
printf 'Source: afudesk\nSection: net\nPriority: optional\nMaintainer: AfuDesk <support@afudesk.app>\nStandards-Version: 4.6.2\n\nPackage: afudesk\nArchitecture: any\nDescription: AfuDesk remote desktop\n' > "$stage/debian/control"
deps=$(cd "$stage" && find opt/afudesk -type f \( -name '*.so' -o -name '*.so.*' -o -name afudesk \) -print0 |
  xargs -0 dpkg-shlibdeps -O --ignore-missing-info -l"$stage/opt/afudesk/lib" | sed -n 's/^shlibs:Depends=//p')
test -n "$deps"
arch=$(dpkg --print-architecture)
size=$(du -sk "$stage/opt/afudesk" | cut -f1)
cat > "$stage/DEBIAN/control" <<EOF
Package: afudesk
Version: 0.1.0
Section: net
Priority: optional
Architecture: $arch
Maintainer: AfuDesk <support@afudesk.app>
Installed-Size: $size
Depends: $deps, dbus, gnome-keyring
Description: AfuDesk remote desktop
 Simple remote desktop for Linux X11 sessions.
EOF
cat > "$stage/usr/bin/afudesk" <<'EOF'
#!/bin/sh
export LD_LIBRARY_PATH="/opt/afudesk/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
exec /opt/afudesk/afudesk "$@"
EOF
chmod 755 "$stage/usr/bin/afudesk"
cat > "$stage/usr/share/applications/afudesk.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=AfuDesk
Comment=Remote desktop
Exec=afudesk
Terminal=false
Categories=Network;RemoteAccess;
StartupWMClass=com.afu.afudesk
EOF
desktop-file-validate "$stage/usr/share/applications/afudesk.desktop"
# Build tooling metadata is not part of the installed package.
rm -r -- "$stage/debian"
dpkg-deb --root-owner-group --build "$stage" "$out/afudesk_0.1.0_${arch}.deb"
dpkg-deb --info "$out/afudesk_0.1.0_${arch}.deb" > "$out/deb-info.txt"
dpkg-deb --contents "$out/afudesk_0.1.0_${arch}.deb" > "$out/deb-contents.txt"
