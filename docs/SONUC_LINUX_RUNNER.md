# Linux runner sonucu — 2026-11-11

Bu dosya ilk runner tesliminin tarihsel kaydıdır. Güncel Secret Service bulunmadığında şifreli fallback, CI düzeltmeleri ve gerçek test/paket sonuçları [LINUX_TEST_RAPORU.md](LINUX_TEST_RAPORU.md) ve [CIHAZ_GUVENI.md](CIHAZ_GUVENI.md) içindedir. Artifact artık `.deb` ve kanıt dizinini içerir; ayrıca açılmış bundle yüklenmez. Windows analiz/test adımları da hata kodlarını ayrı doğrular.

## DONE

- `app/linux/`: Flutter GTK runner, Flutter/Cargokit plugin ve native asset bundle kurulumu; Windows runner değişmedi.
- `afudesk/secure_storage`: libsecret Secret Service üzerinde `read`/`write`/`clear`; servis veya anahtarlık sorunu tek cümle Türkçe hata döndürür. Token loglama ve plaintext fallback yok. Native roundtrip ve erişilemeyen servis testleri eklendi.
- Mevcut `ci.yml` içine `ubuntu-latest`, Flutter 3.44.9, Rust stable, GTK/xcap/Enigo/gilrs/libsecret bağımlılıkları; cargo test/check, Flutter analyze/test/build eklendi. Windows/WAN job içerikleri korundu.
- `scripts/linux/build.sh`: yalnız Linux build-name 0.1.0/build-number 1 geçersiz kılması; pubspec/protokol/Windows sürümü değişmedi.
- `scripts/linux/package.sh`: `.deb` 0.1.0, `/opt/afudesk`, `/usr/bin/afudesk` library yolu wrapper'ı, desktop entry; runtime bağımlılıkları bundle içindeki ELF dosyalarından `dpkg-shlibdeps` ile üretilir. Secret Service sağlayıcısı gnome-keyring dahil.
- `scripts/linux/smoke.sh`, `input_receiver.py`, `rust/examples/linux_smoke.rs`: izole Xvfb + D-Bus içinde gerçek xcap PNG, Enigo fare koordinatı/klik/metin/Enter olaylarının ayrı Tk alıcısında doğrulanması; kurulu release uygulamasının pencere oluşturması, beş saniye ayakta kalması, screenshot, WM_DELETE_WINDOW ile zamanında temiz kapanması. Timeout başarı sayılmaz. CI paket kurulum/kaldırmasını da kontrol eder.
- CI artifact adımı `always()` ile paket, bundle, PNG ve smoke loglarını toplar.

## Yerel gerçek doğrulama

Windows PowerShell oturumunda çalıştırıldı:

```text
shellcheck --shell=bash scripts/linux/build.sh scripts/linux/package.sh scripts/linux/smoke.sh
PASS: ShellCheck/bash parser (exit 0)
rustfmt --edition 2021 --check rust/examples/linux_smoke.rs
PASS: rustfmt (exit 0)
Python ast.parse(input_receiver.py), yaml.safe_load(ci.yml), Linux/Windows job ve always artifact assertions
PASS: Python AST, YAML Linux/Windows job, always artifact (exit 0)
git diff --check
PASS (exit 0)
```

## FAILED / ortam kısıtı

Git Bash `bash -n` denemesi Windows sandbox ortamında `CreateFileMapping ... Win32 error 5` ile başlamadan durdu (exit 256). Betiklerin parser/statik doğrulaması native ShellCheck ile başarıyla yapıldı; `bash -n` CI içinde ayrıca çalışacak.

## UNVERIFIED

- Bu Windows makinede GTK/Linux çalıştırılmadı; Linux build, libsecret native testi, .deb üretimi/kurulumu ve Xvfb smoke sonuçları henüz yok. Linux'ta başarılı çalışmış gibi raporlanamaz.
- Rust cargo check/test koordinatöre bırakıldı; yerel rustfmt Linux cfg kodunun tip kontrolü değildir.
- CI çalışmasının bu dalda gönderildikten sonra mevcut workflow üzerinden `workflow_dispatch --ref feat/linux-ubuntu` ile başlatılması gerekir.
- Secret Service dışında şifreli dosya fallback uygulanmadı. Wayland uzak kontrol davranışı bu X11 smoke testinin kapsamında doğrulanmaz.
- Commit/push yapılmadı.
