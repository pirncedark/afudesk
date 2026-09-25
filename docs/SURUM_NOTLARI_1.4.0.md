**English** · [Türkçe](#türkçe)

## AfuDesk v1.4.0 — registered devices: no code next time

### New
- **Devices remember each other:** after the first connection with a code and password is accepted, the connecting side finds the computer under **Registered devices** on the home screen and connects with **Connect** — no code, no password. Works across different internet connections, like any AfuDesk connection.
- **Always asked, even for registered devices:** the other side still has to accept every time. The request dialog shows a "Registered device" badge.
- **Listens while the app is open:** a registered device can ask to connect even if the "Give access" screen is closed; the request dialog appears on any screen, and after accepting the session screen (with **Disconnect**) opens by itself.
- **Remove / forget:** the sharing side can remove a device under **Trusted devices** — that device then needs a code again. The connecting side can **Forget** a computer.
- **Found even if the IP changes:** on the local network via mDNS, over the internet via address lookup.

### Security
- Each device has a permanent key that is verified on every connection. The host stores only a hash of the pairing token, bound to that device's key — a copied token is useless on another device.
- The connection code now works only while the "Give access" screen is open.
- Lists show only the device name and when it was last seen; no IP addresses or keys.

### Compatibility
- Protocol version 4: both sides must use v1.4.0.

---

## Türkçe

## AfuDesk v1.4.0 — kayıtlı cihazlar: bir dahaki sefere kod yok

### Yeni
- **Cihazlar birbirini hatırlar:** kodla ve parolayla yapılan ilk bağlantı kabul edilince, bağlanan taraf bilgisayarı ana ekranda **Kayıtlı cihazlar** altında bulur ve **Bağlan**'a basar — kod yok, parola yok. Her AfuDesk bağlantısı gibi farklı internet bağlantıları arasında da çalışır.
- **Kayıtlı cihaz için de her seferinde onay:** karşı taraf yine her seferinde kabul eder. Onay penceresinde "Kayıtlı cihaz" rozeti görünür.
- **Uygulama açıkken dinler:** "Bağlantı ver" ekranı kapalı olsa da kayıtlı cihaz bağlanmak isteyebilir; onay penceresi hangi ekranda olunursa olsun çıkar, kabul edilince oturum ekranı (**Bağlantıyı kes** ile) kendiliğinden açılır.
- **Kaldır / Unut:** bağlantı veren taraf cihazı **Güvenilen cihazlar** altından kaldırabilir — o cihaz bir dahaki sefere yine kod ister. Bağlanan taraf bilgisayarı **Unut**abilir.
- **IP değişse de bulunur:** yerel ağda mDNS ile, internette adres aramasıyla.

### Güvenlik
- Her cihazın her bağlantıda doğrulanan kalıcı bir anahtarı vardır. Host eşleşme jetonunun yalnız özetini, o cihazın anahtarına bağlı saklar — kopyalanan jeton başka cihazda işe yaramaz.
- Bağlantı kodu artık yalnız "Bağlantı ver" ekranı açıkken geçerlidir.
- Listelerde yalnız cihaz adı ve son görülme gösterilir; IP ya da anahtar gösterilmez.

### Uyumluluk
- Protokol sürümü 4: iki taraf da v1.4.0 kullanmalı.
