## AfuDesk v1.0.0 — İlk sürüm

İki bilgisayarı **sunucusuz** bağlayan uzak masaüstü. Hesap yok, ücret yok, aracı sunucu yok.

### Neler var
- **Bağlantı ver:** tek kod + 6 haneli parola; kod 10 dakika geçerli ve tek kullanımlık.
- **Bağlan:** kodu yapıştır, parolayı yaz; karşı taraf onaylayınca ekranı gör.
- **Onay penceresi:** kim bağlanıyor gösterilir; fare/klavye kontrolü ayrı izin. 60 sn içinde yanıt yoksa otomatik red.
- **Uzak kontrol:** fare (sol/sağ/orta tık, tekerlek), klavye (Türkçe harfler, Ctrl kısayolları).
- **Akıllı görüntü:** yalnız değişen ekran bölgeleri gönderilir; 2560 pikselden geniş ekranlar otomatik küçültülür.
- **Ağ:** aynı ağda her zaman; internet üzerinden UPnP ya da IPv6 ile. Uygulama hangi durumda olduğunu açıkça yazar.
- **Güvenlik:** Argon2id + XChaCha20-Poly1305 şifreli kod, QUIC/TLS 1.3, oturum başına sertifika ve parmak izi sabitleme.
- **Afu Uygulamaları:** AfuDM, AfuTube, AfuRemote kısayolları.

### Bilinen sınırlar
- Operatör CGNAT kullanıyorsa ya da çift modem varsa ve UPnP/IPv6 yoksa internet üzerinden bağlanılamaz (aynı ağda çalışır).
- Pano ve dosya aktarımı bu sürümde yok.
- Android sürümü (izleyici) sonraki sürümde.
- İlk açılışta Windows Güvenlik Duvarı izin isteyebilir: **Özel ağlar** için izin ver.

### Kurulum
`AfuDesk-windows-x64.zip` dosyasını indir, çıkar, `afudesk.exe`'yi çalıştır. SHA-256 değeri `AfuDesk-windows-x64.zip.sha256` dosyasında.

### Test
Rust çekirdeğinde 38 test (gerçek QUIC uçtan uca dahil), 27 arayüz testi, gerçek Windows motoru üzerinde 2 uçtan uca entegrasyon testi (gerçek ekran yakalama + bağlantı + onay + kesme) ve 9 ekranlık görsel denetim.

---

**English:** First release of AfuDesk — a serverless remote desktop. One password-encrypted code, direct QUIC connection, approval dialog with separate control permission. Works on the same network and over the internet with UPnP or IPv6. Download `AfuDesk-windows-x64.zip`, extract, run `afudesk.exe`.
