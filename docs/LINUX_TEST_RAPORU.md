# Linux ve EK4 test raporu — 2026-11-12

Kaynak kod: e8d2710. Tam CI43100024620 ve Ubuntu24.04 CI43100030810 sürüyor. Önceki e42873f koşusunda core/bridge/Flutter release geçti; smoke hedefi ortak Cargo çıktı dizini hatasıyla durdu. Son düzeltme iki lockfile'ın derleme dizinlerini ayırır.

| Kontrol | Durum | Kanıt |
|---|---|---|
| Windows hedefinde Rust çekirdek | DONE | core-final.txt: 202 unit + 29 integration = 231 passed, 0 failed; 10 gercek_ masaüstü testi mevcut CI kuralıyla koşulmadı |
| Windows hedefinde hesap/FFI köprüsü | DONE | bridge-final.txt: 19 passed, 0 failed |
| 30 günlük sabit süre, rollback, account/client/key/tamper | DONE | trust-red.txt → trust-green.txt: 7/7 |
| Mock HTTP PKCE login, JWT/nonce, refresh, rotation, invalid_grant | DONE | core-final.txt içindeki google_oauth testleri; üretim HTTP kapısı reddedilir |
| Anahtar iptali, bağımsız yerel onay ve izin koruma | DONE | common-security-green.txt; pair-race-red.txt → pair-race-green.txt; gerçek loopback QUIC iptal testi |
| Nonce içermeyen refresh kanıtı | DONE | peer-refresh-red.txt → peer-refresh-green.txt; unpaired/foreign/expired/aud/iss/missing/stale/revoked/duplicate reddi |
| Dinleme/publish ve IP içermeyen davet | DONE | policy-green.txt, share-green.txt, core-final.txt; gerçek şifre çözme testi |
| Script/CI/config sözdizimi | DONE | shellcheck-final.txt, static-final.txt, apt-sources-green.txt (4/4) |
| Linux şifreli fallback ve dört süreç atomik erişim | DONE | e8d2710 CI43100024620/43100030810 native kasa adımları PASS; tam ham log koşu bitince alınır |
| Linux core, bridge ve native Wayland type-check/link | DONE | e42873f CI43094930051: 225 core + 19 bridge; Flutter Linux release PASS. Gerçek Wayland runtime anlamına gelmez |
| Flutter analyze ve tüm arayüz testleri | DONE | e42873f Windows/Linux ayrı analyze adımları ve 229 arayüz testi PASS; yerel SDK cache yazma izni yok |
| Linux release | DONE | e42873f CI43094930051/43094934655 Flutter build linux release PASS |
| Xvfb kare/input/Flutter pencere | DONE | a50404e, CI https://github.com/pirncedark/afudesk/actions/runs/43111365389 linux job success |
| .deb üretim, kurulum, ldd ve kaldırma | DONE | a50404e, CI https://github.com/pirncedark/afudesk/actions/runs/43111365389 linux job success |
| Windows DPAPI/release regresyonu | DONE | CI43094930051 DPAPI, core231, bridge19, analyze, Flutter229 ve release PASS; job yalnız artifact servis hatası nedeniyle FAILURE. e8d2710 tekrar koşusu sürüyor |
| Gerçek Wayland compositor/Google hesabı/Tailscale/Proxmox | UNVERIFIED | Uygun gerçek donanım ve hesap oturumu yok; sunucuya bağlanılmadı |

## CI hata zinciri

| Run | Gerçek sorun | Düzeltme |
|---|---|---|
| 43044412814 / 43045698075 | Ubuntu archive/security/azure DNS çözülemedi; apt 8 dakika sınırında durdu | 3699d72: erişilebilir HTTPS mirror; imza doğrulaması korunur |
| 43088529164 | Fazla geniş Ubuntu URI eşleşmesi Microsoft Signed-By anahtarını çakıştırdı | ade41c1: yalnız Ubuntu host/mirror URI'leri; 4 regresyon testi |
| 43088933771 | Linux Gercek fabrikası Windows ViGEm tipini KolSurucusu olarak kullanıyordu | 18bb308: Windows cfg; Linux'ta desteklenmiyor hatası |
| 43090168311 | Beklenen RED: Secret Service yokken Clear/write/read yok; exit10 | 18bb308 + 6e61bc5: şifreli dosya, süreç kilidi, sticky backend, atomik key |
| 43091510180 / 43091518909 | Native kasa geçti; Rust linker -lgbm bulunamadı | 809aaf4: libgbm-dev |
| 43092549123 / 43092555090 | Flutter analyze iki çok satırlı if için curly_braces uyarısı | e42873f: braces; Windows toplu pub/analyze/test yerine ayrı adımlar |
| 43094930051 / 43094934655 | Smoke example E0277: ortak Cargo hedefinde iki lockfile'ın farklı anyhow çıktıları çakıştı | e8d2710: global CARGO_TARGET_DIR kaldırıldı; bağımsız varsayılan dizinler |
| 43094930051 artifact / 43097791540 startup | GitHub artifact API HTML yanıtı ve “An unexpected error has occurred...” | Ham servis kanıtı korundu; 5a02bb4 dar kapsamlı artifact ve beş dakika sınırı; yeni koşular |

Ham loglar reports/linux/ci-*.txt altındadır. Yerel `pause-red.txt` Flutter SDK izin hatasıdır; davranışsal RED olarak kullanılmaz. Windows'ta çalışan taşınabilir testler Linux cfg runtime başarısı yerine geçmez. Xvfb de Wayland kanıtı değildir.

Gerçek sınırlamalar [EKSIKLER.md](EKSIKLER.md), mimari [CIHAZ_GUVENI.md](CIHAZ_GUVENI.md), tam dosya listesi [DEGISEN_DOSYALAR.md](DEGISEN_DOSYALAR.md).
