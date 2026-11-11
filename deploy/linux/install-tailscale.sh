#!/usr/bin/env bash
# Run on the future Ubuntu/Debian machine, never on the build workstation.
set -euo pipefail
[[ $(uname -s) == Linux ]] || { echo 'Bu betik yalnız Linux içindir.' >&2; exit 1; }
[[ $EUID -eq 0 ]] || { echo 'sudo bash deploy/linux/install-tailscale.sh kullanın.' >&2; exit 1; }
# shellcheck source=/dev/null
. /etc/os-release
case "${ID}:${VERSION_CODENAME:-}" in
  ubuntu:noble|debian:bookworm|debian:trixie) ;;
  *) echo 'Bu dağıtım için resmi Tailscale paket sayfasını kullanın.' >&2; exit 1 ;;
esac
apt-get update
apt-get install -y ca-certificates curl gnupg
install -d -m 0755 /usr/share/keyrings
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
base="https://pkgs.tailscale.com/stable/${ID}/${VERSION_CODENAME}"
curl --proto '=https' --tlsv1.2 -fsSL "${base}.noarmor.gpg" -o "$work/key.gpg"
curl --proto '=https' --tlsv1.2 -fsSL "${base}.tailscale-keyring.list" -o "$work/tailscale.list"
gpg --batch --show-keys "$work/key.gpg" >/dev/null
grep -Fq 'signed-by=/usr/share/keyrings/tailscale-archive-keyring.gpg' "$work/tailscale.list"
install -m 0644 "$work/key.gpg" /usr/share/keyrings/tailscale-archive-keyring.gpg
install -m 0644 "$work/tailscale.list" /etc/apt/sources.list.d/tailscale.list
apt-get update
apt-get install -y tailscale
systemctl enable --now tailscaled
echo 'Kuruldu. Belgede cihazınıza uygun tailscale up adımına geçin; erişim kuralları değiştirilmedi.'
