# Telefon kolu doğrulama raporu

Dal: `ozellik/telefon-kol`
Kapı raporunun çalışma ağacı `97cb6c5` üzerine uygulanmış, commit öncesi değişiklikleri içeriyordu.

- `flutter_rust_bridge_codegen generate`: başarılı.
- `cargo test --release`: başarılı, 72 test geçti; 0 başarısız. Dokümantasyon testleri de geçti.
- `flutter analyze`: başarılı, sorun bulunmadı.
- `flutter test`: başarılı, 63 test geçti.
- `python scripts/kapi_dogrula.py`: OYUN_MODU 7/7 tamamlandı; tüm denetimde 15/17 tamamlandı. Çıkış `NOT_READY` çünkü bu işle ilgisi olmayan `donanim_h264` ve `otomatik_cozunurluk` maddeleri hâlâ TODO.
- GitHub CI: başarılı (Rust, Flutter, Windows derlemesi), [run 36085864655](https://github.com/pirncedark/afudesk/actions/runs/36085864655).
- GitHub Android: başarılı (APK derlemesi ve imza doğrulaması), [run 36085870255](https://github.com/pirncedark/afudesk/actions/runs/36085870255).
- Android APK: `app/build/ci-artifact/AfuDesk-android.apk`, 59,279,790 bayt. SHA-256: `cc5e49880370655bddd3b84d9375c3e4bca2c417626f7599d5880e3f4ede748a`; `.sha256` dosyasıyla eşleşti.

Ayrıntılı çıktılar: `cargo-test-release.txt`, `flutter-analyze.txt`, `flutter-test.txt`, `kapi-dogrula.txt`, `test-results.json` ve `AUDIT.md`.

Gerçek Bluetooth kolu ve host'taki `joy.cpl` denemesi bağlı telefon/Windows host gerektirdiğinden bu ortamda yapılmadı; adımlar README'de yer alıyor.

Merge, etiket ve yayın yapılmadı. Sürüm numarası değiştirilmedi.
