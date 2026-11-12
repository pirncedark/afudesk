# Linux kurulum/güvenlik belgeleri — sonuç

Tarih: 2026-11-11. Kapsam: yalnız belge ve incelenebilir örnek config/betik; commit/push veya gerçek sunucu bağlantısı yapılmadı. Sunucu henüz yoktur. Başka ajanların dosyaları değiştirilmedi.

## DONE — teslim edilenler

- `docs/UBUNTU_KURULUM.md`: Proxmox fiziksel host + Ubuntu Server 24.04 VM + Ubuntu Desktop istemcisi ayrımı, sabit cihaz adları, imzalı Tailscale apt deposu, `.deb` kurulum sözleşmesi, Xorg yolu ve Wayland/portal/PipeWire eksikleri.
- `docs/LINUX_SUNUCU.md`: yerel konsol ve başarılı yeni anahtarlı Tailscale/OpenSSH oturumu önkoşulu; Ed25519, parola/root reddi, yalnız tail IP `ListenAddress`, Ubuntu `ssh.socket` geçişi, Proxmox `LISTEN_IP` ve tam restart, UFW/IPv6, fail2ban, Ubuntu ile Debian/Proxmox'a ayrı unattended-upgrades, dar API token ve sertifika kontrolü, uygulanmayan ACL örneği, kurtarma ve gerçek cihaz testleri.
- `deploy/linux/install-tailscale.sh`: Ubuntu noble / Debian bookworm / trixie için HTTPS üzerinden resmi anahtar + signed-by apt deposu; dağıtım ve root/Linux kontrolü. `curl|sh` yok; `tailscale up`, netfilter veya ACL değişikliği otomatik yapılmaz.
- `deploy/linux/00-afudesk-ssh.conf.example`: yalnız tail IPv4 ve anahtar girişi; yerine konulması gereken adres şablonu içerir, doğrudan kurulmaz.
- `deploy/linux/fail2ban-sshd.local`: systemd journal kullanan SSH jail.
- `deploy/linux/52-afudesk-ubuntu-security.conf`: yalnız Ubuntu security origin; otomatik reboot yok.
- `deploy/linux/52-afudesk-proxmox-security.conf`: yalnız Debian security origin; PVE/Proxmox/kernel blacklist; major/kernel otomatik güncelleme yok.
- `deploy/linux/20-afudesk-auto-upgrades.conf`: günlük apt güvenlik güncelleme zamanlaması.
- `deploy/linux/tailscale-acl.example.json`: yönetici grubundan etiketli host 22/8006 ve VM 22 için dar örnek; mevcut ACL değiştirilmedi.

Tailscale'in varsayılan netfilter kabul zincirinin UFW'den önce çalışabilmesi belgede açıkça belirtilir. UFW durum çıktısından tailnet içindeki diğer portların kapalı olduğu çıkarılmaz. Mevcut Tailscale erişim kuralları ve netfilter modu korunur. Proxmox firewall/bridge etkileşimi, SSH açılışında Tailscale adresinin hazır olması ve sertifika SAN eşleşmesi garanti edilmiş gibi sunulmaz.

## Gerçek yerel doğrulama

Windows çalışma istasyonunda yalnız statik kontroller yapıldı. ShellCheck sürümü `0.11.0`:

```text
PASS ACL JSON structure (Tailscale policy engine not run)
PASS fail2ban INI structure (fail2ban-client not run)
PASS SSH template directives (Linux sshd not run)
PASS UBUNTU_KURULUM.md: 4 bash blocks, ShellCheck warning/error level
PASS LINUX_SUNUCU.md: 12 bash blocks, ShellCheck warning/error level
PASS install-tailscale.sh: ShellCheck all levels
PASS 52-afudesk-ubuntu-security.conf delimiter check only (apt-config not run)
PASS 52-afudesk-proxmox-security.conf delimiter check only (apt-config not run)
PASS 20-afudesk-auto-upgrades.conf delimiter check only (apt-config not run)
```

JSON Python `json.loads`, INI `configparser`, şablon zorunlu SSH direktifleri ve UTF-8 çözümlemesiyle kontrol edildi. Belge bash blokları geçici dosyalara çıkarılıp native `shellcheck --shell=bash --severity=warning` ile incelendi; geçici dosyalar kaldırıldı. Installer `shellcheck --shell=bash deploy/linux/install-tailscale.sh` ile tüm seviyelerde geçti.

İlk `bash -n` denemesi **FAILED — ortam engeli**: Git Bash `CreateFileMapping ... Win32 error 5` ile başlayamadı. Bu Linux bash çalışma testi olarak raporlanmaz. Native ShellCheck parser/statik analizi başarıyla kullanılabildi.

## UNVERIFIED — gerçek Linux/donanım gerekir

- `.deb` artifact'ının başarılı CI derlemesi, apt ile kurulumu, masaüstünden açılışı.
- Xorg ekran/klavye/fare, çoklu ekran, ses ve gerçek uçtan uca oturum.
- `sshd -t/-T`, `ss` ile yalnız tail IP dinleme, socket→service geçişi ve reboot.
- Proxmox web/API TLS zinciri + ad doğrulaması, `pveproxy`/`spiceproxy` restart ve listen kontrolü.
- IPv4/IPv6 LAN dış erişim reddi, tailnet içindeki yetkisiz cihazın reddi, Proxmox bridge/VM ağı.
- Fail2ban gerçek journal eşleşmesi, ban kuralı ve engelleme.
- `apt-config dump`, `unattended-upgrade --dry-run --debug` ile origin, blacklist, timer ve yeniden başlatma davranışı. Config delimiter kontrolü apt söz dizimi kanıtı değildir.
- Proxmox `pveum` komutlarının hedef sürümde uygulanması, tek VM API okuması, başka VM 403, token iptali.
- ACL örneğinin Tailscale policy engine'de mevcut policy ile test edilmesi. JSON parse, Tailscale semantik doğrulaması değildir. Örnek hiçbir hesaba uygulanmadı.
- Google Console yayın durumu, OAuth ve AfuDesk'in ayrı 30 günlük cihaz güveni bu belge görevinin dışında doğrulanır; biri diğerinin ömrü yerine geçmez.

## Kaynak doğrulama sınırı

Resmî Canonical, Proxmox ve Tailscale kaynakları belgelerde ilgili adımlara bağlantılıdır. Agent-reach/Jina okuma denemesi yerel ağ proxy'sinde `127.0.0.1` bağlantı hatası verdi; resmî kaynaklar web aracı üzerinden doğrulandı. Bazı resmî sayfalar 403/503 döndürdüğünde Proxmox'un resmî kaynak deposu ve Canonical'ın resmî security/discourse açıklaması kullanıldı. Sunucuya bağlanılmadı; servis veya tailnet ayarı uygulanmadı.

Yeni PR açılmadı. Mevcut PR'lar #17 ve #7 konusunda karar koordinatöre bırakıldı.
