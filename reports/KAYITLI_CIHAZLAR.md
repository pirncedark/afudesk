# Kayıtlı cihazlar — doğrulama (iroh'a uyarlandı)

Tarih: 2026-09-25 · Dal: `ozellik/kayitli-cihazlar` (main v1.3.0 birleştirildi)

## Ne yapıldı
- Kalıcı cihaz kimliği: iroh anahtarı (host ve izleyici ayrı dosya); açık anahtar her bağlantıda el sıkışmada doğrulanır.
- Eşleşme: kodla yapılan ve onaylanan ilk oturumda host izleyiciye 256 bit jeton verir; host yalnız SHA-256 özetini, izleyicinin kimliğine bağlı saklar.
- Kayıtlı bağlantı: kod/parola yok, **her seferinde onay**; kaldırılan cihaza "Bu cihaz sizi kaldırdı…", yanlış jeton ya da başka cihaz reddedilir; izleyici kaydı bu durumlarda kendiliğinden silinir.
- Arka planda dinleme: uygulama açıkken host çalışır; kod yalnız "Bağlantı ver" ekranı açıkken geçerli. Onay penceresi uygulama kökünden, hangi ekranda olunursa olsun.
- Yerel ağ keşfi: mDNS (`_afudesk._udp`), cihaz kimliğiyle (IP değişse de); internet üzerinden iroh adres araması + relay.
- Arayüz: ana ekranda "Kayıtlı cihazlar" (ad + son görülme, Bağlan / Unut), "Bağlantı ver"de "Güvenilen cihazlar" (Kaldır), onay penceresinde "Kayıtlı cihaz" rozeti. IP/jeton/kimlik gösterilmez.
- Android: kayıtlar `path_provider` uygulama destek klasöründe.

## Sonuçlar
- `cargo test --release`: 92/92 geçti (15'i kayıtlı cihazlara özel; `gercek_mdns_kimlikle_bulunur` gerçek çoklu yayınla).
- `flutter analyze`: sorun yok. `flutter test`: 73/73 geçti (8'i kayıtlı cihazlara özel).
- `python scripts/kapi_dogrula.py`: KAYITLI_CIHAZLAR 6/6, FASTPATH 6/6, OYUN_MODU 7/7, WAN 4/4; SONRAKI'de H.264 ve otomatik çözünürlük TODO (v1.3.0 ile aynı).

## Sınırlar
- Gerçek iki bilgisayar arasında arayüzden elle deneme yapılmadı; uçtan uca testler gerçek iroh bağlantısıyla (127.0.0.1) çalışır.
- Telefon (Android) yalnız bağlanan taraf; Android'de arka planda dinleme yok.
