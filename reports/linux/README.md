# Doğrulama kanıtları

`LINUX_TEST_RAPORU.md` güncel sonuçların dizinidir. Bu klasör hem önceki başarısız denemeleri hem gerçek RED→GREEN çıktılarını korur; her dosya başarı kanıtı değildir.

| Dosya | Anlamı |
|---|---|
| core-final.txt / bridge-final.txt | Windows üzerinde son ortak Rust testleri: 231 / 19 PASS |
| trust-red.txt → trust-green.txt | Kontrol karakterli istemci kimliği reddi; yedi güven testi |
| peer-refresh-red.txt → peer-refresh-green.txt | Nonce'suz refresh JWT yalnız eşleşmiş anahtar ve canlı aynı hesap kaydıyla |
| pair-race-red.txt → pair-race-green.txt | Google beklemesi sırasında silinen yerel key onayı geri oluşturulmaz |
| common-permissions-red.txt / common-security-green.txt | Ortak Google key izni, ilk izinleri koruma ve gerçek QUIC iptali |
| apt-sources-red.txt → apt-sources-green.txt | Microsoft Signed-By anahtarını koruyan dört Ubuntu URI testi |
| pause-red.txt | Yerel Flutter SDK cache yazma izni hatası; davranışsal RED değildir |
| ci-43044412814.txt / ci-43045698075.txt | Kullanıcının istediği iki eski FAILURE: Ubuntu DNS/apt |
| ci-43094930051-linux-final-fetch.txt | Gerçek tam Linux job logu: test/build PASS, shared Cargo hedefinde anyhow çakışması; artifact servis hatası |
| ci-43094930051-linux.txt / ci-43094934655-linux.txt | Log API erişim girişiminin Server Error JSON yanıtı; compiler logu veya başarı kanıtı değildir |
| github-startup-error.txt | GitHub başlangıç hatasının kamuya açık run sayfasındaki asıl servis açıklaması |
| *-state.json | O anda alınmış CI durum snapshot'ı; güncel durum yerine geçmez |
| shellcheck-final.txt / static-final.txt / actionlint-final.txt | Betik, config ve workflow statik kontrolleri |

Eski bazı `.txt` dosyaları PowerShell UTF-16 çıktısıdır; yeni dosyalar UTF-8'dir. Native stderr yönlendirmesinde PowerShell `NativeCommandError` sarmalı bulunabilir; gerçek test sonucu ve process çıkış kodu esas alınır. Yerel Windows testleri Linux cfg/runtime kanıtı değildir. Xvfb X11 kanıtıdır, Wayland veya gerçek Google hesabı kanıtı değildir.
