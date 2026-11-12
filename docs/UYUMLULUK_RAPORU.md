# Windows / Ubuntu uyumluluk raporu

| Alan | Windows | Ubuntu/Linux |
|---|---|---|
| Protokol | SURUM=8, ALPN afudesk/8; değişmedi | Aynı |
| Ağ | Mevcut direct/NAT/relay, legacy kod/parola varsayılanları korunur | Tailscale arayüzüne, yoksa loopback'e bind; wildcard/UPnP/mDNS yok; relay yedek |
| Paylaşım | Mevcut davet yolu korunur | Paylaşım yükünde doğrudan IP yok; literal IP relay URL'leri çıkarılır |
| Google erişimi | EK4 ortak cihaz güveni ve bağımsız yerel key onayı | Aynı hesap + canlı Drive kaydı + gerçek QUIC anahtarı + yerel key onayı; üretimde legacy parola reddedilir |
| Güven süresi | Ayrı imzalı 30 günlük kayıt; süre dolunca yeniden giriş | Aynı; Google token ömrünü uzatmaz |
| Saklama | Mevcut CurrentUser DPAPI | libsecret; yoksa şifreli 0600 dosya/key ve 0700 dizin |
| Ekran/girdi | WGC/xcap + Enigo | X11 xcap + Enigo; gerçek Xvfb test hattı |
| Wayland | Uygulanmaz | Tek portal/PipeWire backend'i; gerçek compositor runtime UNVERIFIED |
| Ses/gamepad | Mevcut Windows backend'leri | Linux host backend yok; FAILED/desteklenmiyor |
| Tepsi/autostart/terminal/güncelleyici | Mevcut Windows runner | Linux ek native özellikler tamamlanmadı; FAILED/desteklenmiyor |
| Paket | Mevcut Windows release build | .deb v0.1.0, içinde release bundle; kurulum/kaldırma CI hattı |

Gerçek derleme, test ve paket sonuçları [LINUX_TEST_RAPORU.md](LINUX_TEST_RAPORU.md)'dedir; tablo tek başına runtime başarı iddiası değildir. Gerçek iki cihaz/Google/Tailscale/Proxmox ve eski Windows ikilisiyle çapraz sürüm denemeleri UNVERIFIED. Windows ağ varsayılanları ve protokol korunur; yalnız açıkça istenen ortak EK4 Google güvenlik akışı genişletildi.
