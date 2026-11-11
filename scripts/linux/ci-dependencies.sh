#!/usr/bin/env bash
# Ephemeral CI runners only. APT still verifies Ubuntu's signed repository data.
set -euo pipefail
source /etc/os-release
: "${VERSION_CODENAME:?Ubuntu codename missing}"
printf 'Ubuntu suite: %s\n' "$VERSION_CODENAME"
mirror=''
for candidate in https://archive.ubuntu.com/ubuntu https://mirrors.edge.kernel.org/ubuntu https://mirrors.mit.edu/ubuntu; do
  if curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' \
      --connect-timeout 5 --max-time 15 "$candidate/dists/$VERSION_CODENAME/InRelease" -o /dev/null \
    && curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' \
      --connect-timeout 5 --max-time 15 "$candidate/dists/$VERSION_CODENAME-security/InRelease" -o /dev/null; then
    mirror=$candidate
    break
  fi
done
if [[ -z "$mirror" ]]; then
  printf 'No Ubuntu HTTPS mirror is reachable.\n' >&2
  exit 1
fi
printf 'APT mirror: %s (signature verification unchanged)\n' "$mirror"
# Replace only Ubuntu entries, including runner mirror+file: sources. Keep
# Signed-By, components, suites and unrelated repositories intact.
sudo python3 - "$mirror" <<'PY'
import pathlib, re, sys
mirror = sys.argv[1]
for path in [pathlib.Path('/etc/apt/sources.list'), *pathlib.Path('/etc/apt/sources.list.d').glob('*.list'), *pathlib.Path('/etc/apt/sources.list.d').glob('*.sources')]:
    if not path.exists():
        continue
    text = path.read_text()
    if path.suffix == '.sources':
        blocks = re.split(r'(\n\s*\n)', text)
        for i, block in enumerate(blocks):
            if re.search(r'^URIs:.*(?:ubuntu|apt-mirrors)', block, re.M):
                blocks[i] = re.sub(r'^URIs:.*$', 'URIs: ' + mirror, block, flags=re.M)
        text = ''.join(blocks)
    else:
        text = re.sub(r'(?m)^(deb(?:-src)?\s+(?:\[[^\]]*\]\s+)?)(\S*(?:ubuntu|apt-mirrors)\S*)(\s+)', lambda m: m[1] + mirror + m[3], text)
    path.write_text(text)
PY
options=(-o Acquire::Retries=1 -o Acquire::http::Timeout=20 -o Acquire::https::Timeout=20)
sudo apt-get "${options[@]}" -o APT::Update::Error-Mode=any update
sudo env DEBIAN_FRONTEND=noninteractive apt-get "${options[@]}" install -y --no-install-recommends \
  clang libclang-dev cmake ninja-build pkg-config libgtk-3-dev \
  libsecret-1-dev libudev-dev libdbus-1-dev libpipewire-0.3-dev \
  libxcb1-dev libxcb-randr0-dev libxcb-shm0-dev libxcb-xfixes0-dev \
  libxrandr-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev \
  libegl1-mesa-dev libgl1-mesa-dri libglu1-mesa-dev libxdo-dev \
  xvfb xauth xdotool scrot openbox python3-tk dbus-x11 gnome-keyring \
  dpkg-dev desktop-file-utils
