# Arka plan host — sonuç / devam kaydı (tur 2/5)

Durum: kod düzenlemeleri mevcut; derleme, test ve gerçek cihaz kabulü yapılmadı.

## Dosyalarda uygulananlar

- Desktop OAuth offline onayı ve yenileme uç noktası; invalid_grant → G6,
  geçici ağ hatası → kasa korunur. Refresh paketi CurrentUser DPAPI sınırına bağlandı.
- Açılışta Windows kasasından sessiz yükleme ve 60 saniyelik çevrimiçi kayıt.
- Windows tepsisi, X ile gizleme, --arka-plan, tek örnek koruması.
- Varsayılan kapalı başlangıç ayarı, HKCU Run komutu ve mevcut opt-in yolunu
  uygulamanın yeni konumundan ilk açılışta güncelleme.
- Google bağlantısı için 1/2/4 saniyelik üç yeni deneme; eski çekirdek resume
  döngüsü kapatılarak sınırsız deneme önlenir. Kullanıcının kesmesi tekrar denemez.
- Kasaya yazma/silme sıralandı; yükleme sonrası saklama hatası görünmez yetkili
  hesap bırakmamak için yerel duraklatmaya gider. Durdurma hatasında UI temizlenir.
- İlk kabul süresi uygulama belleğinde ölçülür, token veya adres loglanmaz.
- Saf komut/bekleme/tepsi testleri ve sahte kasayla gerçek servis yükleme,
  ağ hatası, geçersiz token, durdurma hatası ve üç denemeli bağlantı testleri eklendi.
- Eski elle kurulum betiğinin başlangıç argümanı --arka-plan olarak düzeltildi.

## Gerçek doğrulama çıktısı

Kullanıcının son talimatı yalnız dosya düzenlemeye izin verdiği için:

- cargo test (rust/): ÇALIŞTIRILMADI.
- cargo test (app/kopru/): ÇALIŞTIRILMADI.
- Flutter test/analyze/Windows build: ÇALIŞTIRILMADI; SDK varlığı kontrol edilmedi.
- Yerel ağ ilk kabul/ilk kare süresi: ÖLÇÜLMEDİ, gerçek cihaz oturumu açılmadı.
- Run kaydı / zamanlanmış görev / uygulama / betik: ÇALIŞTIRILMADI, KURULMADI.
- Sürüm numarası ve paralel terminal/protokol/oturum dosyaları değiştirilmedi.

Salt okunur git kontrolü: `git diff --check` çıkış kodu 0, çıktı boş.
`git diff --stat` (izlenen dosyalar; yeni dosyalar bu özete dahil değildir):

```text
 app/kopru/src/api/google.rs             | 107 ++++++++++++++++-
 app/lib/hesap/gercek_hesap_servisi.dart | 197 +++++++++++++++++++++++++++++---
 app/lib/hesap/google_hatasi.dart        |   5 +-
 app/lib/hesap/hesap_sayfasi.dart        |   4 +
 app/lib/main.dart                       |  17 ++-
 app/lib/sayfalar/ayarlar.dart           |  38 ++++++
 app/windows/runner/CMakeLists.txt       |   2 +-
 app/windows/runner/flutter_window.cpp   | 148 +++++++++++++++++++++++-
 app/windows/runner/flutter_window.h     |   9 +-
 app/windows/runner/main.cpp             |  19 ++-
 rust/src/google_oauth.rs                |  90 ++++++++++++++-
 scripts/arka_plan_kur.ps1               |   3 +-
 12 files changed, 604 insertions(+), 35 deletions(-)
```

Yeni dosyalar: `app/lib/arka_plan/arka_plan.dart`,
`app/lib/hesap/oturum_kasasi.dart`, `app/test/arka_plan_test.dart`,
`docs/ARKA_PLAN.md`, `docs/ARKA_PLAN_SONUC.md`.

## Kalan kabul işleri

1. Android: mevcut GoogleHesap.kt yalnız açık GetSignInWithGoogleOption
   kullanıyor; otomatik seçim/sessiz açılış uygulanmış değil. Android dosyaları
   görevdeki yazma izin listesinde bulunmadığından değiştirilmedi. Dart'tan
   açılışta mevcut giris yöntemini çağırmak gereksiz hesap penceresi açacağından
   yapılmadı. Native GetGoogleIdOption/auto-select ve iptalde sessiz geri dönüş
   için ayrı Android kapsamı gerekiyor; token yenileme de bu akışa bağlanmalı.
2. Gerçek Desktop OAuth refresh yanıtının anahtarları bağlayan nonce claim'ini
   koruduğunu doğrula. Mevcut güvenlik doğrulaması gevşetilmedi; imzalı nonce
   yoksa yenileme reddedilir. Sağlayıcıyla doğrulanmadan kalıcı oturum kabulü
   tamamlandı sayılamaz.
3. İki Rust test takımı, Flutter test/analyze ve Windows derlemesi.
4. Gerçek cihazda yeniden başlatma, tepsi/Explorer yeniden başlatma, internet
   kesintisi, üç deneme, kullanıcı iptali ve Google erişimini kaldırma senaryoları.
5. LAN'da ilk kabul ve ilk kare sürelerini ölç; bu belgeye gerçek değerleri ekle.
6. Uygulama dosyası taşındıktan sonra yeni exe'nin ilk açılışında Run değerini
   kontrol et. Eski exe henüz açılmadan yolun otomatik bulunması garanti değil.

Chrome Uzak Masaüstü'nü kaldırma kararı bu kabul kontrollerinden sonra verilmeli.
