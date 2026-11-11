# Linux çalışma kaydı — 2026-11-11

Kaynak: `_gorev/2026-11-11/AFUDESK_LINUX.md`, tüm ekler ve koordinatör cevabı okundu.
Başlangıç: f5fa79cb8f3e9a861a8b8dea350acdb2d194e151, feat/linux-ubuntu.

1. İnceleme, Linux runner/CI/paket ve güvenlik belgeleri.
2. Linux ağ kısıtları, IP içermeyen paylaşım, eşleşmiş cihaz kontrolü ve testleri.
3. Linux doğrulamasından sonra EK 4: mevcut OAuth/oturum mekanizmasını genişletme.
4. Testler, kod inceleme, commit/push, SONUC.

Kararlar:
- ALPN afudesk/8 ve SURUM=8 korunur.
- Windows ağ varsayılanları korunur; Linux güvenli politika ayrı cfg ile uygulanır.
- Açık PR #17 ve #7 bulundu: yeni PR açılmaz, mevcut PR değiştirilmez; kullanıcı kararına bırakılır.
- Flutter PATH'te yok, koordinatör uyarınca Linux doğrulaması GitHub Actions üzerinde denenir.
- Bağımsız Linux runner/CI ve sunucu belgeleri paralel çalışma becerisiyle ayrıldı.
- Çalışmayan veya koşulamayan hiçbir özellik DONE sayılmaz.
- Ortam: asıl linked-worktree Git index'i F:/afuproject/AfuDesk/.git altında ve yazmaya kapalı. Git add/commit `index.lock: Permission denied` döndü. Ana metadata'ya yazmak yerine bu worktree içindeki `.linux-publish/` bağımsız Git metadata alanı kullanılır; başlangıç objeleri yalnız okunur. Asıl worktree index/HEAD değişmez, yayın commit'i ayrı metadata'da izlenir.
- İlk commit 53a8d72; bağımsız metadata'dan origin/feat/linux-ubuntu'ya push başarılı. Schannel kimlik hatası ve Git Bash helper sandbox hatası; process-only OpenSSL + mevcut gh kimliği header'ı (gizli, loglanmaz/commitlenmez) ile çözüldü.
- İlk CI: https://github.com/pirncedark/afudesk/actions/runs/43041600452 . Linux bağımlılık/derleme aşaması bekleniyor. EK4 ajanı yalnız inceleme ve RED test hazırlığı yapıyor; üretim değişikliği Linux sonucu bekliyor.
- Yerel Windows cargo check core ve bridge başarılı. `cargo test -- --skip gercek_`: 209 passed, 0 failed; 10 gerçek masaüstü testi mevcut CI kuralıyla koşulmadı.
- İnceleme iki Important buldu: yeniden bağlantıda ilk izinleri yükseltme ve yerel grant silindikten sonra açık oturumu sürdürme. RED regresyonları kaydedildi; izinler yerel grant'e eklenip yükleniyor, 1 saniyelik kontrol yerel grant iptalinde QUIC bağlantısını kapatıyor. İlgili 9 Linux politika/güvenlik testi GREEN. Son tam testler EK4 sonrası tekrar koşulacak.
- Linux doğrudan soketinde tail /10 veya loopback /8 dışında varsayılan UDP rota yok; genel adres keşif/QAD çıkışı için Tailscale source IP'si kullanılmaz. Windows bind ayarları değişmedi.
- Güvenlik düzeltmeleri 30ed8f2 ile push edildi; reviewer iki Important bulguyu kapattı, yeni Critical/Important yok. Gerçek QUIC revoke testi dahil ilgili 10 test geçti.
- İlk Linux apt aşaması 15+ dakika sürünce kontrol amaçlı 8 dakika apt sınırı + 30sn ağ timeout/retry1 ve yalnız Linux dispatch seçeneği eklendi (e358d83). İkinci CI: https://github.com/pirncedark/afudesk/actions/runs/43044412814 (wan=linux). İlk run Windows build devam ettiği için henüz iptal edilmedi.
- İlk run Windows job full success: DPAPI; 209 Rust test, 224 Flutter test, analyze, release .exe. Windows tamamlanınca Linux apt işi iptal edildi, log alındı: Ubuntu archive/security/azure DNS çözümlenemiyor; compiler aşamasına ulaşılmadı. Raw kanıt `reports/linux/ci-linux-base.txt`, `ci-windows-base.txt` (gh api job logs; gh run --log sandbox dışı cache yazdığı için kullanılamıyor).
- Sıra kararı: kullanıcı metni önce Linux işini yapmayı istiyor, başarılı CI'yı EK4 için açık önkoşul koymuyor. Linux altyapısı/güvenliği ve gerçek build girişimi önce yapıldı; dış DNS engeli FAILED/UNVERIFIED kaydedildi. Bağımsız EK4 üretimine şimdi geçilir; Linux build başarılı diye raporlanmaz.
- EK4 ortak Windows+Linux şartı uyarınca yalnız Google giriş kolunda bağımsız cihaz key onayı/izinleri/revoke ortaklaştırıldı (Windows RED regresyonu `common-permissions-red.txt`). Eski Windows kod/parola ve ağ varsayılanları aynı kalır; bu Google akışının görevde açık istenen güvenlik genişletmesidir. Protokol değişmez.
- CI kanıtları `reports/linux-ci/` ayrı dizine alındı: depodaki yerel Windows logları Linux artifact'ında başarı kanıtı gibi görünemez. Yeni run yalnız o koşunun çıktısını içerir. Desteklenen Ubuntu24.04 için `wan=ubuntu24` koşusu 43045698075; varsayılan Linux job ubuntu-latest olarak korunur.


## Devam oturumu — 02fad08 sonrasından

- Kullanıcının 30 dosyası korundu; silme/stash ve yasaklı Git komutları kullanılmadı. Yayın yine `.linux-publish` metadata alanıyla yapılır; açık PR17/7 ve main değişmez.
- İki önceki FAILURE ham logu alındı. DNS sorunu HTTPS Ubuntu mirror seçimiyle giderildi; Microsoft depo anahtarını koruyan URI regresyonları 4/4 geçti (3699d72, ade41c1).
- EK4 önceki yarım kodu tamamlandı: yanlış tip refresh yanıtı ve kontrol karakterli client ID RED→GREEN; imzalı sabit 30 gün, rotation, ayrı cihaz revoke, bağımsız key onayı. Yerel core231 + bridge19 PASS.
- Bağımsız inceleme: duraklatma/geri yükleme yarışı, nonce'suz refresh peer kanıtı ve onay doğrulaması sırasında key iptalini geri oluşturma düzeltildi. Sonuncu için atomik yerel grant kontrolü ve RED→GREEN regresyon var.
- Linux Secret Service eksikken şifreli fallback RED CI43090168311 exit10 ile doğrulandı. AES-GCM0600, bounded flock, sticky backend, tek GTK instance, atomik no-replace key publish eklendi. İncelemenin Important bulguları kapandı.
- Native Wayland backend CI'da type-check aşamasını geçti; link için libgbm-dev gerektiği görüldü ve 809aaf4 ile düzeltildi. Gerçek Wayland runtime halen UNVERIFIED.
- Son kod CI43092549123 (Windows+ubuntu-latest) ve CI43092555090 (Ubuntu24.04) ile doğrulanıyor; sonuç test raporuna yazılacak.

### Sonraki CI adımları ve dış engel (20:50 UTC)

- 43092549123/43092555090 Linux native core225 + bridge19 PASS; Flutter analyze iki çok satırlı if için curly_braces uyarısı verdi. Windows toplu analiz/test adımı analyze çıkışını sonraki test başarısıyla örtüyordu. e42873f iki if'i düzeltti, Windows pub/analyze/test adımlarını ayırdı. CARGO_TARGET_DIR ortaklaştırıldı.
- e42873f koşuları: 43094930051 (Windows+ubuntu-latest), 43094934655 (Ubuntu24.04). Her iki Linux'ta Rust/bridge ve Flutter analyze/test/release adımları PASS. Sonraki `cargo build --release --manifest-path rust/Cargo.toml --example linux_smoke` adımı bir saniyede FAILURE. Hata metni henüz alınamadı; disk veya başka neden tahmin olarak kalır, kesin neden diye raporlanmaz.
- Bu iki koşuda `always()` artifact adımı bundle da topladığı için uzun süre in_progress kaldı. Job log API HTTP502 Server Error, run-log archive API404 döndü. 43094934655 için normal cancel ve force-cancel kabul edildi; durum henüz tamamlanmadı. Windows e42873f koşusunda ayrı analyze/test ve release derleme adımları PASS, artifact yüklemesi sürüyor.
- 5a02bb4 artifact'ı yalnız .deb/kanıt dizinine daralttı, beş dakika sınırı ekledi; smoke derlemesi stdout/stderr ve df çıktısı dosyaya alınır. 43097791540 işler başlamadan startup_failure verdi (0 job). Resmî actionlint1.7.12 Windows binary'si checksum doğrulanarak .ci-tools altında kullanıldı, YAML/ifadeler PASS. .ci-tools yalnız yerel metadata exclude alanında, commit edilmez.
- Aynı 5a02bb4 tekrar 43098421996 ile tetiklendi, şu an queued/0job. Kullanıcıya startup_failure sayfasındaki asıl hata metni asenkron soruldu; API ayrıntıyı vermiyor, tarayıcı hostu kullanılamıyor. Gerçek GitHub sebebi görülmeden billing/kota/YAML nedeni uydurulmaz.
- Paket/Xvfb hâlâ UNVERIFIED; ekran kanıtı veya .deb varmış gibi yazılmaz. Açık PR17/7 ve main korunur. GUNLUK ve dış SONUC final kayıtları bitişte yazılacak; doküman/kanıt değişiklikleri henüz commit edilmedi.

### Log erişimi geri geldi — 21:05 UTC

- Kamuya açık GitHub run sayfası 43097791540 için gerçek açıklamayı verdi: “An unexpected error has occurred and we've been automatically notified.” İstek kimliği e7cb0b3b-de10-11f1-8351-f1b39d55524f; kanıt reports/linux/github-startup-error.txt. Kullanıcının eski asenkron soruyu yanıtlaması gerekmiyor.
- 43094930051 tam Linux logu sonunda alındı: reports/linux/ci-43094930051-linux-final-fetch.txt. Smoke build hatası disk değil, ortak CARGO_TARGET_DIR içinde iki bağımsız lockfile'ın farklı anyhow sürümlerinin çıktılarının çakışmasıdır. e8d2710 global hedef dizini ayarını iki job'dan da kaldırdı; bağımlılık sürümleri değiştirilmedi.
- Aynı eski koşunun Windows ve Linux artifact yüklemeleri beş denemeden sonra HTML yanıtı JSON olarak okuyamadı. Bu gerçek servis hatasıdır; artifact boyutu kanıtlanmış kök neden değildir. 43094934655 iptali sonunda tamamlandı. Eski hatalı 43098421996 da iptal edildi.
- e8d2710 push edildi. Yeni tam Windows+ubuntu-latest CI43100024620 ve Ubuntu24.04 CI43100030810 başladı; native şifreli fallback ve dört süreç testleri iki Linux'ta PASS. Sonuçlar aşağıya eklenecek.
