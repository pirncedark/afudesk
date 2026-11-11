# Wayland backend sonucu — 2026-11-11

Güncel doğrulama: native Linux core/bridge ve Flutter release derlemeleri CI43094930051/43094934655'te geçti. Gerçek Wayland compositor runtime hâlâ UNVERIFIED; Xvfb bu kanıtın yerine geçmez. Son durum için [LINUX_TEST_RAPORU.md](LINUX_TEST_RAPORU.md).

Aşağıdaki ilk inceleme ve yerel denemeler tarihsel kayıttır; o sırada Linux derlemesi henüz doğrulanmamıştı.

## Kaynak değişiklikleri

- `rust/src/platform.rs`: yalnız Linux'ta `XDG_SESSION_TYPE=wayland` veya dolu `WAYLAND_DISPLAY` tespitinde portal fabrikasına yönlendirme. X11 xcap/Enigo yolu ve Enigo feature'ları aynı kaldı. Zaman damgası yardımcısı yalnız crate içinde görünür yapıldı.
- `rust/src/linux_portal.rs`: RemoteDesktop `CreateSession` → `SelectDevices`; aynı session ile ScreenCast `SelectSources`; RemoteDesktop `Start`; ScreenCast `OpenPipeWireRemote` zinciri. ScreenCast için ayrı izin oturumu açılmaz. Kullanıcı klavye ve işaretçi erişimini birlikte vermediyse oturum kapalı kalır.
- Portal FD'si yalnız izinli PipeWire remote'a bağlanır. Seçili stream'in logical_size/size koordinatları ve node'u input bildirimlerinde kullanılır; varsa pipewire-serial ile `target.object` tercih edilir. Varsayılan/unrestricted PipeWire core'a bağlanılmaz.
- Input mevcut `Girdi` ve tuş adı eşlemesini kullanır: mutlak fare, Linux evdev fare düğmeleri, dikey/yatay discrete scroll, keysym basma/bırakma ve Unicode metin. Mikrofonun mevcut ayrı uygulama yolu korunur. uinput/root veya izinsiz fallback yok.
- Kareler yalnız bir `FrameSlot` içinde son değer olarak tutulur. RGB/RGBA/RGBx/BGRA/BGRx → opaque RGBA dönüşümünde boyut, piksel üst sınırı, offset, chunk size, pozitif stride, satır dolgusu ve taşmalar kontrol edilir; diğer formatlar işlenmez.
- Yakalama ve input tutacakları aynı weak-cache oturumunu paylaşır. Son tutacak bırakıldığında stop bayrağı, kareleri silme, Session.Close, PipeWire sonlu iterate/quit/disconnect ve iki worker join adımları vardır. Portalın `Closed` sinyali ve worker hataları da kareleri silerek input'u durdurur. Actor/PipeWire worker guard'ları panic durumunda da stop/kare temizliği yapar.
- İzin çağrılarının yanıtına çağrıdan önce abone olunur; reddetme/iptal/timeout tek cümle Türkçe hata verir. Request.Close ve Session.Close süre sınırlıdır. Input kuyruğu 8 komutla sınırlıdır; süresi geçmiş komut uygulanmaz. Token/FD/secret loglanmaz.
- `rust/Cargo.toml`: yalnız Linux hedefi altında zbus 5, pipewire 0.10, xkeysym 0.2. Zbus mevcut default async-io davranışını korur; Tokio/libei feature'ı genel olarak etkinleştirilmedi. İki lock dosyasında yalnız çekirdeğin yeni doğrudan bağımlılık kayıtları eklendi, paket sürümleri değiştirilmedi.
- Koordinatör `lib.rs` içine `#[cfg(any(target_os = "linux", test))] mod linux_portal;` ekledi; native alt modül yalnız Linux'ta derlenir.

## DONE — gerçek yerel doğrulama

Windows PowerShell; `CARGO_TARGET_DIR=<worktree>/target-linux-validation`:

```text
cargo test --offline --manifest-path rust/Cargo.toml --lib linux_portal::tests -- --nocapture
running 5 tests
test linux_portal::tests::invalid_layout_is_rejected_before_allocation ... ok
test linux_portal::tests::portal_denial_and_cancellation_fail_closed ... ok
test linux_portal::tests::selected_stream_uses_logical_not_pixel_coordinates ... ok
test linux_portal::tests::stride_padding_offset_and_bgr_conversion ... ok
test linux_portal::tests::latest_frame_overwrites_and_close_discards_all_frames ... ok
test result: ok. 5 passed; 0 failed; 0 ignored

cargo check --offline --manifest-path rust/Cargo.toml
Finished dev profile in 2.93s
cargo check --offline --manifest-path app/kopru/Cargo.toml
Finished dev profile in 3.97s

rustfmt --edition 2021 --check rust/src/linux_portal.rs rust/src/platform.rs
exit 0
git diff --check
exit 0
```

Testler gerçek portal izni değil; yanıt kodu, kare sınırları/dönüşümü, seçili logical koordinatlar ve kapanmış son-kare slotu davranışının testleridir. Core/bridge check Windows hedefindedir. Native-only son iyileştirmelerden sonra format/diff yeniden geçti. Zbus'un ekstra Tokio feature'ı son incelemede kaldırıldı; koordinatör nihai seri Cargo kontrolünü çalıştıracak.

## FAILED — ortam kilidi

Son tekrar test derlemesi, ortak target içindeki test executable başka bir iş tarafından açıkken Windows `LINK: fatal error LNK1104 ... afudesk_core-b332a1e71b4e6ee3.exe dosyası açılamıyor` ile durdu. Hiçbir süreç öldürülmedi ve kaynaklar bunun için değiştirilmedi. Önceki iki portable koşu 5/5 geçti; koordinatör ayrıca common-security-green koşusunda bu 5 testi dahil ederek 15/15 PASS bildirdi. Koordinatör ortak target üzerinde ek Cargo çağrılarını durdurdu, nihai kontrolü seri yapacak.

## UNVERIFIED / kalan doğrulama

- Native Linux zbus/PipeWire bölümü burada type-check edilmedi. Windows `cargo check` sonucu Linux API uyumluluğu kanıtı değildir.
- Gerçek Wayland izin verme/red/iptal, compositor tarafından session kapatma, PipeWire bağlantısı ve kapanışta gerçek Close/quit/join davranışları çalıştırılmadı. Bunlar uygun Linux Wayland oturumunda doğrulanmalı.
- Xvfb testi X11'i doğrular; Wayland runtime testi yerine geçmez. Linux CI run `43044412814` bu çalışma sırasında apt/DNS aşamasındaydı; koordinatör Ubuntu 24.04 runner denemesini yönetiyor.
- Native backend tüm session için klavye+işaretçi+bir monitör izni ister; yalnız ekran izni verilirse daha düşük yetkili bir fallback açılmaz.
- Commit/push yapılmadı; bu görevin kapsamı dışındaki dosyalara yazılmadı.

API kaynakları: [RemoteDesktop oturum ve Notify API](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html), [ScreenCast stream/koordinat/PipeWire API](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.ScreenCast.html). Yerel crate kaynaklarından xcap 0.9.8, pipewire 0.10.1, zbus 5.19.0 ve enigo 0.6.1 API'leri incelendi.
