# Görev: Linux runner, CI, paket ve gerçek ekran testi
ONAY: Kullanıcı kapsamı onayladı; doğrudan uygula, ek onay sorma.
Çalışma dizini: C:\Users\afuuu\Desktop\afuproject\_wt\afudesk-linux
Önce AGENTS.md oku. Başka ajanlar aynı kod tabanında; onların değişikliklerini geri alma, kendi işini uyumlu tut.
Sahiplik: app/linux/** (yeni), .github/workflows/ci.yml Linux job eklemesi, scripts/linux/** (build/package/smoke), rust/examples/linux_smoke.rs (yeni). Bu liste dışına yazma. Commit/push YOK.
YASAK: restore/checkout --/reset/clean/stash/rebase/amend/push --force/switch; main merge; görünür pencere; diğer worktree/ana depoya yazmak.

📍 C:\Users\afuuu\Desktop\afuproject\_wt\afudesk-linux\app\rust_builder\linux\CMakeLists.txt:14 apply_cargokit — Rust FFI zaten Linux destekli ama app/linux runner yok. GTK Flutter Linux runner oluştur (normal template uyumlu), mevcut app ve Windows runner değişmesin.
📍 C:\Users\afuuu\Desktop\afuproject\_wt\afudesk-linux\.github\workflows\ci.yml:31 jobs/test — Windows job korunur; ubuntu-latest Linux job ekle, Flutter 3.44.9 ve Rust stable, xcap/enigo/gilrs/GTK/secret bağımlılıkları. cargo test, köprü cargo check, flutter analyze/test/build linux. Yeni workflow yerine mevcut ci.yml'e ekle: workflow_dispatch --ref feat/linux-ubuntu ile çalıştırılacak.
📍 C:\Users\afuuu\Desktop\afuproject\_wt\afudesk-linux\rust\src\platform.rs:40 Fabrika, :186 XcapYakalayici — gerçek Xvfb ekranını yakalayan Rust smoke örneği yaz, PNG kaydet, gerçek Enigo fare/klavye davranışını mümkünse doğrula. Flutter release uygulamasını aynı Xvfb'de başlatıp sağlıklı ayakta ve pencere oluştuğunu doğrula; temiz kapat, timeout'u başarı sayma. xdotool/screenshot ile uygulama kanıtı da al. Kanıtları her durumda CI artifact.
📍 C:\Users\afuuu\Desktop\afuproject\_wt\afudesk-linux\app\pubspec.yaml:23 version 1.10.16+30 — Linux .deb v0.1.0 paketle; bundle ve runtime bağımlılıklarını ekle. Kurulum resmi /opt/afudesk, desktop entry; çalıştırılabilir wrapper doğru library yolunu kullansın. Paket artifact üret. Sürüm/protokol Windows'ta değişmesin.

Native storage MethodChannel için runner'a sonraki entegrasyon noktası bırak: afudesk/secure_storage. Şimdilik eksikse dürüstçe bildir, gizli plaintext fallback yapma.
Doğrula: yerelde mümkün statik kontroller (Python/YAML/bash varsa), Rust check koordinatör yapacak. Log ve DONE/FAILED/UNVERIFIED özeti docs/SONUC_LINUX_RUNNER.md dosyasına yaz. Linux çalıştırmış gibi raporlama.
