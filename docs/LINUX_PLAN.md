# AfuDesk Linux/Ubuntu v0.1.0

İnceleme tabanı: f5fa79cb8f3e9a861a8b8dea350acdb2d194e151. Tüm görev ekleri okundu.
ALPN `afudesk/8`, `SURUM=8` ve mevcut Windows ağ varsayılanları korunur.

| Dosya:satır (başlangıç sürümü) | Sorun | Linux çözümü / sınır |
|---|---|---|
| rust/src/platform.rs:48,186 | Windows WGC; diğer masaüstlerinde xcap | X11 xcap/XCB ile ekran; mevcut Enigo x11rb ile fare/klavye. Xvfb gerçek kare ve input testi. |
| rust/src/platform.rs:66,216 | Enigo varsayılanı X11; Wayland portal onayı yok | linux_portal.rs ile tek ScreenCast+RemoteDesktop/PipeWire izin oturumu eklendi. Native Linux core/bridge ve Flutter release CI'da derlendi; gerçek compositor/izin/input davranışı UNVERIFIED. Xvfb Wayland kanıtı değildir. |
| rust/src/sanal_mikrofon.rs:1,29; host.rs:212 | VB-CABLE/WASAPI yalnız Windows | cfg(windows) mevcut; Linux ses/mikrofon backend'i yok, eksik raporlanır. |
| rust/src/sanal_kol.rs:22; platform.rs:72 | ViGEm yalnız Windows | Linux gamepad host backend'i yok; destek varmış gibi gösterilmez. |
| rust/src/terminal.rs:1; platform.rs:46 | Win32 terminal pencere/odak entegrasyonu | Linux terminal özelliği kapalı, masaüstü X11 input bundan bağımsız. |
| app/windows/runner/*; app/lib/main.dart:49 | Tepsi, launcher, autostart Windows runner'da | app/linux GTK runner eklenir. Linux tepsi/autostart uygulanmış sayılmaz; url_launcher Linux sistem tarayıcısını açar. |
| app/lib/guncelleme.dart:90; bilesenler/guncelle_karti.dart:55 | Güncelleyici Windows/Android paket formatına özel | Linux .deb kurulum hattı; otomatik güncelleme eksik. Windows kodu korunur. |
| app/rust_builder/linux/CMakeLists.txt:14 | Rust FFI plugin var, uygulama Linux runner yok | GTK/CMake Linux runner + aynı cargokit köprüsü. |
| .github/workflows/ci.yml:31 | Yalnız Windows ve Android CI | Mevcut CI'ya ubuntu-latest Rust/Flutter/Xvfb/.deb/artifact job'u eklenir. |
| rust/src/ag.rs:86,95,113,197,304 | Wildcard UDP, UPnP, mDNS, adres yayını | Linux tailscale0 gerçek arayüz IP'sine bind; yoksa loopback. UPnP ve mDNS kapalı; env ile açık arayüze geçilemez. DNS/pkarr kimlik yayını açılmaz. n0 relay yedek kalır. |
| rust/src/host.rs:172,295,739; kod.rs:30 | Google dizini ve davet doğrudan IP taşıyabilir | Google dizininde yalnız tail adresi; paylaşılabilir davette adres yok, relay alanında IP literal yok. Windows davet varsayılanı aynı. |
| rust/src/host.rs:891,903,940 | Kod/parola ve başka hesaba manuel onay yolu | Linux üretim oturumlarında yalnız doğrulanmış Google + cihaz anahtarı + yerel cihaz onayı; legacy parola yolu reddedilir. Yerel ağsız test modu ayrı. |
| rust/src/google_oauth.rs:35,153,252,348 | PKCE/loopback ve refresh zaten mevcut | Aynı OAuth istemcileri korunur; yeni istemci oluşturulmaz. EK4 için 30 gün AfuDesk imzalı cihaz güveni ayrı eklenir. |
| app/kopru/src/api/google.rs:74,138,286 | Kalıcı refresh bundle var, 30 gün güven süresi/imza yok | EK4 ortak imzalı cihaz güveni, yerel çapa, değişmeyen 30 gün sınırı, rotation/revoke ve yeniden giriş uygulanır; ayrıntı CIHAZ_GUVENI.md. |
| app/lib/hesap/gercek_hesap_servisi.dart:46,49 | Linux kasa/client ID etkin değil | Linux Secret Service MethodChannel + mevcut desktop OAuth yolu genişletilir. DPAPI Windows'ta kalır. |

Sıra: Linux derleme/güvenlik → EK4 → bütün testler → branch commit/push → PR durumu.
Mevcut açık PR #17 ve #7 bulundu: yeni PR açılmaz, başka PR'a ekleme yapılmaz.
Donanım yok: SSH/Proxmox/Tailscale ACL yalnız belge ve örnek yapılandırmadır.

Kanıt kaydı: `docs/LINUX_CALISMA_KAYDI.md`, `reports/linux/`, `docs/LINUX_TEST_RAPORU.md`.
