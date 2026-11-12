# Eksikler ve dış doğrulama sınırları

1. UNVERIFIED: Wayland tek ScreenCast+RemoteDesktop/PipeWire backend'i uygulandı; gerçek compositor üzerinde izin verme/red/iptal, kare, input ve kapanış denenmedi. Xvfb yalnız X11 kanıtıdır.
2. FAILED (destek yok): Linux ses/mikrofon ve sanal oyun kolu host backend'leri. Windows implementasyonları korunur; Linux gamepad isteği anlaşılır hata verir.
3. FAILED (destek yok): Linux tepsi/autostart, terminal pencere modu ve otomatik paket güncelleyici. Normal masaüstü bağlantısı bunlardan bağımsızdır.
4. UNVERIFIED: Gerçek Google hesabıyla Windows/Linux tarayıcı girişi, gerçek refresh rotation ve Google Console yayın durumu. Yeni OAuth istemcisi/secret/Console ayarı oluşturulmadı. External/Testing + Drive kapsamı yedi günlük refresh token sınırına tabidir; ayrı 30 günlük AfuDesk güveni bunu uzatmaz. [Google belgesi](https://developers.google.com/identity/protocols/oauth2#expiration).
5. UNVERIFIED: Fiziksel Ubuntu 24.04/Proxmox, gerçek Tailscale direct/relay geçişi, ACL/SSH/API/fail2ban/unattended-upgrades runtime kontrolleri. Donanım yok; sunucuya bağlanılmadı ve mevcut Tailscale kuralları değiştirilmedi.
6. UNVERIFIED: Eski yayımlanmış Windows ikilisiyle çapraz sürüm runtime testi. SURUM=8/afudesk/8 ve Windows legacy ağ/kod/parola davranışı korunur; ortak EK4 Google güvenlik genişletmesi güncel iki platformda uygulanır.
7. UNVERIFIED (kullanıcı kararı): PR #17 ve #7'ye dokunulmadı; yeni PR açılmadı, main'e merge yapılmadı. Linux dalının PR yolu SONUC'ta sorulacak.

EK4 üretim kodu ve Secret Service bulunmadığında şifreli 0600 fallback artık eksik değildir. Derleme/test sonuçları [LINUX_TEST_RAPORU.md](LINUX_TEST_RAPORU.md), tasarım [CIHAZ_GUVENI.md](CIHAZ_GUVENI.md) içindedir. Yayın metadata'sı `.linux-publish`; asıl linked-worktree index/HEAD'e yazılmaz.
