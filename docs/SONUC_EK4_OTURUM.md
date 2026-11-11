# EK4 oturum ve cihaz güveni — devam sonucu

DONE: ortak Windows/Linux üretim uygulaması ve yerel Rust testleri. Son Linux/Flutter CI sonucu LINUX_TEST_RAPORU.md'de; gerçek Google hesabı/Console UNVERIFIED.

- `cihaz_guveni.rs`: host iroh anahtarıyla imzalanmış host/viewer/sub/client/session/time bağlı tam 30 gün. Sliding expiry yok; rollback, tamper ve yabancı hesap/anahtar/istemci reddedilir.
- `api/google.rs`: bağımsız yerel çapa, imzasız desktop migration için yeniden giriş, refresh/rotation korunması, mevcut Drive üyeliğiyle restore/heartbeat, ayrı cihaz revoke. Silinen kayıt sessizce yeniden oluşturulmaz; duraklatma yalnız çevrimdışına geçer.
- `google_oauth.rs`: private mock HTTP ile PKCE exchange/refresh/rotation/invalid_grant; üretim Google HTTPS, JWT imza/audience/issuer/nonce kontrolleri korunur.
- `google_baglanti.rs`, `google.rs`, `google_drive.rs`, `host.rs`: nonce içermeyen refresh JWT yalnız bağımsız yerel eşleşmiş QUIC anahtarı ve canlı aynı hesap Drive kaydıyla kabul edilir; nonce uydurulmaz. İptal edilen key atomik kontrolle geri oluşturulamaz; ilk izinler yükseltilmez.
- Flutter: Linux desktop giriş/kasa yolu, G6 yeniden giriş, rotation kaydı, çıkış/duraklatma yarışları için kontroller/testler.
- Linux kasa: libsecret; servis yoksa sticky AES-256-GCM dosya+key0600/dizin0700, owner/no-follow/nlink denetimi, bounded flock, atomik session yazımı ve renameat2 no-replace key publish. Eski keyring oturumu geri gelemez. Windows DPAPI korunur.

Gerçek yerel komut/çıktı:

```text
cargo test --offline --manifest-path rust/Cargo.toml -- --skip gercek_
202 unit + 29 integration passed; 0 failed; 10 gerçek masaüstü testi mevcut CI kuralıyla koşulmadı.
cargo test --offline --manifest-path app/kopru/Cargo.toml --lib
19 passed; 0 failed.
```

Kanıtlar: reports/linux/core-final.txt, bridge-final.txt, trust-red.txt → trust-green.txt (7/7), peer-refresh-red.txt → peer-refresh-green.txt, pair-race-red.txt → pair-race-green.txt. İmzasız migration RED'i önceki görev kaydındadır. Linux fallback RED CI43090168311 exit10; sonraki CI'da native fallback/dört süreç testleri geçti. Son sürüm kanıtları test raporundadır.

Yerel Flutter girişimi SDK cache yazma iznine takıldı (reports/linux/pause-red.txt); davranışsal RED sayılmaz. Flutter regresyonları CI'da doğrulanır. Bağımsız incelemenin Important bulguları kapatıldı; son key publish incelemesinde ek Critical/Important yok.

Google her refresh'te token döndürmeyebilir; varsa yeni token saklanır. Console Testing/Production ve gerçek hesap girişi UNVERIFIED; kodla değiştirilmedi. Ayrıntı: CIHAZ_GUVENI.md.
