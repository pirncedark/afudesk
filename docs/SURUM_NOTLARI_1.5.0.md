**English** · [Türkçe](#türkçe)

## AfuDesk v1.5.0 — no central directory: your device is published nowhere

### Privacy
- **Your device is no longer published anywhere.** Earlier versions silently wrote each device's identity and relay address to a third-party lookup server on every start. v1.5.0 removes this completely: devices find each other only through the connection code, the saved address of a registered device, or the local network.
- **The relay is only a fallback.** When a direct connection is not possible, traffic can still pass through a relay (it stays end-to-end encrypted). If that relay is ever down, direct connections and same-network connections keep working.
- **Bring your own relay (advanced):** set `AFUDESK_RELAY` to a comma-separated list of relay URLs to use your own; set it to `kapali` to turn the relay off completely.

### Security
- **Longer password:** the one-time password is now 10 digits instead of 6, generated without bias.
- **Wrong-password limit:** after 3 wrong attempts the code is replaced with a new one.
- **Closing "Give access" ends the code immediately.** The old code stops working as soon as the screen closes; an ongoing session is not affected.
- **Flood protection:** the viewer limits how many incoming streams it handles at once.

### Fixes
- If the computer cannot be reached from the internet, "Give access" now says so: "This code only works on the same network." — instead of handing out a code that would silently fail.
- Local-network discovery now also works in relay-only mode.
- Reconnecting after a dropped connection: the host now waits 120 seconds, longer than the viewer keeps retrying, so a returning viewer is no longer turned away at the last moment.
- When the two sides run incompatible versions, the message now says so clearly: "AfuDesk versions don't match; both sides should update."

### Compatibility
- **Protocol version 5: both sides must use v1.5.0.** Older versions cannot connect to v1.5.0 and vice versa.
- Registered devices from v1.4.0 remain registered.
- A registered computer whose internet address changed is now found automatically only on the same network (the central lookup is gone). Over the internet, connect once with a code to refresh it.

---

## Türkçe

## AfuDesk v1.5.0 — merkezi rehber yok: cihazın hiçbir yere yayınlanmaz

### Gizlilik
- **Cihazın artık hiçbir yere yayınlanmıyor.** Önceki sürümler her açılışta cihaz kimliğini ve relay adresini sessizce üçüncü taraf bir adres sunucusuna yazıyordu. v1.5.0 bunu tamamen kaldırdı: cihazlar birbirini yalnız bağlantı kodu, kayıtlı cihazın saklanan adresi ya da yerel ağ üzerinden bulur.
- **Relay yalnız yedek.** Doğrudan bağlantı kurulamazsa trafik yine relay üzerinden geçebilir (uçtan uca şifreli kalır). O relay bir gün kapansa da doğrudan ve aynı ağdaki bağlantılar çalışmaya devam eder.
- **Kendi relay'ını kullan (gelişmiş):** `AFUDESK_RELAY` ortam değişkenine virgülle ayrılmış relay adreslerini yaz; relay'i tamamen kapatmak için `kapali` yaz.

### Güvenlik
- **Daha uzun parola:** tek kullanımlık parola artık 6 değil 10 hane ve yansız üretiliyor.
- **Hatalı parola sınırı:** 3 yanlış denemeden sonra kod yenisiyle değiştirilir.
- **"Bağlantı ver" kapanınca kod hemen biter.** Ekran kapandığı anda eski kod geçersiz olur; süren oturum etkilenmez.
- **Taşma koruması:** izleyici aynı anda işlediği gelen akış sayısını sınırlar.

### Düzeltmeler
- Bilgisayara internetten ulaşılamıyorsa "Bağlantı ver" artık bunu söylüyor: "Bu kod yalnız aynı ağda çalışır." — sessizce çalışmayacak bir kod vermek yerine.
- Yerel ağda bulma artık yalnız-relay modunda da çalışıyor.
- Kopan bağlantıya geri dönüş: host artık 120 saniye bekliyor (izleyicinin yeniden deneme süresinden uzun); geri dönen izleyici son anda reddedilmiyor.
- İki taraf uyumsuz sürümdeyse mesaj bunu açıkça söylüyor: "AfuDesk sürümleri uyuşmuyor; iki taraf da güncellemeli."

### Uyumluluk
- **Protokol sürümü 5: iki taraf da v1.5.0 kullanmalı.** Eski sürümler v1.5.0'a bağlanamaz, v1.5.0 da eskilere bağlanamaz.
- v1.4.0'daki kayıtlı cihazlar kayıtlı kalır.
- İnternet adresi değişen kayıtlı bilgisayar artık yalnız aynı ağdayken kendiliğinden bulunur (merkezi adres araması kaldırıldı). İnternetten bir kez kodla bağlanmak kaydı yeniler.
