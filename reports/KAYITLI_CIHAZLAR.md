# Kayıtlı cihazlar dalı — doğrulama

Tarih: 2026-09-25
Dal: `ozellik/kayitli-cihazlar`
Ana sürüm dosyalarına dokunulmadı; main'e birleştirme, tag ve release yapılmadı.

## Komut sonuçları

- `cargo test --release`: başarılı, 76 test geçti.
- `flutter analyze`: başarılı, sorun bulunmadı.
- `flutter test`: başarılı, 57 test geçti.
- `python scripts/kapi_dogrula.py`: `NOT_READY`.
- `KAYITLI_CIHAZLAR`: 2/4 madde tamamlandı.

## Kapı özeti

Kalıcı kimlik (`kimlik::testler::cihaz_kimligi_kalici`) ve kayıt gidip dönüş/atomik yazma testleri geçti. Kayıtlı cihaz bağlantısı için istenen beş uçtan uca test ve Flutter'daki üç kayıtlı cihaz testi mevcut değil; arayüz maddesi de eksik. Dolayısıyla jetonla bağlantı/onay, kaldırma davranışı, yerel ağ keşfi, arka planda dinleme ve cihaz listesi tamamlanmış değildir.

Ayrıntılı kanıt ve test çıktısı: `reports/test-results.json`, `reports/AUDIT.md`.
