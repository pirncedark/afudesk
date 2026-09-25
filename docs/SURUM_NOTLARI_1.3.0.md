**English** · [Türkçe](#türkçe)

## AfuDesk v1.3.0 — connect across any internet, phone as a gamepad

### New
- **Works across different internet connections, with no setup:** the two computers can be on different networks (home internet, mobile hotspot, another ISP, double NAT). Nobody opens ports, changes router settings or types an IP address — the single code is still all you need. AfuDesk connects directly whenever it can (NAT hole punching) and falls back to an end-to-end encrypted relay only when a direct path is impossible. The relay cannot see the screen, input or files. Tested between a home PC behind double NAT and a machine in a different network: the direct path was established; with direct UDP deliberately blocked, the session ran over the relay with video, mouse and keyboard working.
- **Automatic reconnect:** if the connection drops (for example when the network changes), the viewer reconnects by itself for up to 90 seconds. The host does not have to approve again; only the same device with the session's one-time token can come back.
- **Phone as a gamepad (Android):** use a Bluetooth controller connected to the phone, or the on-screen controls (🎮), as a gamepad on the host. Multi-touch, permission-based.

### Improved
- The session shows when traffic goes through the relay ("aktarmalı").
- Connection errors are short and say what to check, without technical details.
- Fixed: if the host accepted a request very quickly, the approval could be lost and the request timed out after 60 seconds.

### Compatibility
- The connection code format changed (v3). Both sides must use v1.3.0; an older code shows "update both sides".

---

## Türkçe

## AfuDesk v1.3.0 — her internetten bağlantı, telefonla oyun kolu

### Yeni
- **Farklı internet bağlantıları arasında, ayarsız bağlantı:** iki bilgisayar farklı ağlarda olabilir (ev interneti, telefon hotspotu, başka operatör, çift modem). Kimse port açmaz, modem ayarı yapmaz, IP yazmaz — yine tek kod yeterli. AfuDesk mümkünse doğrudan bağlanır (NAT delme); doğrudan yol kurulamazsa yalnız o zaman uçtan uca şifreli bir relay üzerinden bağlanır. Relay ekranı, girdiyi ya da dosyaları göremez. Çift modem arkasındaki bir ev bilgisayarı ile başka ağdaki bir makine arasında denendi: doğrudan yol kuruldu; doğrudan UDP bilerek kapatıldığında oturum relay üzerinden çalıştı, görüntü, fare ve klavye çalıştı.
- **Otomatik yeniden bağlanma:** bağlantı koparsa (ör. ağ değişirse) izleyici 90 saniyeye kadar kendiliğinden yeniden bağlanır. Host'un yeniden onay vermesi gerekmez; yalnız aynı cihaz, oturumun tek kullanımlık jetonuyla geri dönebilir.
- **Telefonla oyun kolu (Android):** telefona bağlı Bluetooth kolu ya da ekrandaki kontrolleri (🎮) host'ta oyun kolu olarak kullan. Çoklu dokunma, izne bağlı.

### İyileştirmeler
- Trafik relay üzerinden geçiyorsa oturumda "aktarmalı" yazar.
- Bağlantı hataları kısa; teknik ayrıntı yerine neye bakılacağını söyler.
- Düzeltme: host isteği çok hızlı kabul ederse onay kaybolup istek 60 saniye sonra zaman aşımına düşebiliyordu.

### Uyumluluk
- Bağlantı kodu biçimi değişti (v3). İki taraf da v1.3.0 kullanmalı; eski kod "iki taraf da güncellemeli" uyarısı verir.
