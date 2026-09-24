**English** · [Türkçe](#türkçe)

## AfuDesk v1.0.0 — First release

A remote desktop that connects two computers **without a server**. No account, no fees, no middleman server.

### What's included
- **Give access:** one code + 6-digit password; the code is valid for 10 minutes and works once.
- **Connect:** paste the code, type the password; see the screen once the other side approves.
- **Approval dialog:** shows who is connecting; mouse/keyboard control is a separate permission. Auto-rejects after 60 seconds without an answer.
- **Remote control:** mouse (left/right/middle click, wheel) and keyboard (including Turkish characters and Ctrl shortcuts).
- **Smart video:** only changed screen areas are sent; screens wider than 2560 px are scaled down automatically.
- **Network:** always works on the same network; over the internet with UPnP or IPv6. The app tells you clearly which case applies.
- **Security:** Argon2id + XChaCha20-Poly1305 encrypted code, QUIC/TLS 1.3, a new certificate per session pinned by fingerprint.
- **Afu apps:** shortcuts to AfuDM, AfuTube and AfuRemote.

### Known limitations
- Cannot connect over the internet behind carrier-grade NAT or double NAT without UPnP/IPv6 (same network still works).
- No clipboard or file transfer in this release.
- The Android viewer comes in the next release.
- On first launch, Windows Firewall may ask for permission: allow **Private networks**.

### Install
Download `AfuDesk-windows-x64.zip`, extract it and run `afudesk.exe`. Windows 10/11 (64-bit). The SHA-256 checksum is in `AfuDesk-windows-x64.zip.sha256`.

### Tests
38 tests in the Rust core (including real end-to-end QUIC), 27 UI tests, 2 end-to-end integration tests on the real Windows engine (real screen capture + connection + approval + disconnect) and a 9-screen visual check.

---

## Türkçe

## AfuDesk v1.0.0 — İlk sürüm

İki bilgisayarı **sunucusuz** bağlayan uzak masaüstü. Hesap yok, ücret yok, aracı sunucu yok.

### Neler var
- **Bağlantı ver:** tek kod + 6 haneli parola; kod 10 dakika geçerli ve tek kullanımlık.
- **Bağlan:** kodu yapıştır, parolayı yaz; karşı taraf onaylayınca ekranı gör.
- **Onay penceresi:** kim bağlanıyor gösterilir; fare/klavye kontrolü ayrı bir izindir. 60 sn içinde yanıt yoksa otomatik reddedilir.
- **Uzak kontrol:** fare (sol/sağ/orta tık, tekerlek), klavye (Türkçe harfler ve Ctrl kısayolları dahil).
- **Akıllı görüntü:** yalnız değişen ekran bölgeleri gönderilir; 2560 pikselden geniş ekranlar otomatik küçültülür.
- **Ağ:** aynı ağda her zaman; internet üzerinden UPnP ya da IPv6 ile. Uygulama hangi durumda olduğunu açıkça yazar.
- **Güvenlik:** Argon2id + XChaCha20-Poly1305 şifreli kod, QUIC/TLS 1.3, oturum başına yeni sertifika ve parmak izi sabitleme.
- **Afu Uygulamaları:** AfuDM, AfuTube ve AfuRemote kısayolları.

### Bilinen sınırlar
- Operatör CGNAT kullanıyorsa ya da çift modem varsa ve UPnP/IPv6 yoksa internet üzerinden bağlanılamaz (aynı ağda çalışır).
- Pano ve dosya aktarımı bu sürümde yok.
- Android izleyici sonraki sürümde.
- İlk açılışta Windows Güvenlik Duvarı izin isteyebilir: **Özel ağlar** için izin ver.

### Kurulum
`AfuDesk-windows-x64.zip` dosyasını indir, çıkar, `afudesk.exe`'yi çalıştır. Windows 10/11 (64 bit). SHA-256 değeri `AfuDesk-windows-x64.zip.sha256` dosyasında.

### Test
Rust çekirdeğinde 38 test (gerçek QUIC uçtan uca dahil), 27 arayüz testi, gerçek Windows motoru üzerinde 2 uçtan uca entegrasyon testi (gerçek ekran yakalama + bağlantı + onay + kesme) ve 9 ekranlık görsel denetim.
