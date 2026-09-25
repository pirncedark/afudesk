# AfuDesk telefon oyun kolu Uygulama Planı

> **Uygulama:** Plan kullanıcı tarafından doğrudan yetkilendirildi; bekletmeden mevcut dalda uygulanır.

**Hedef:** Android'den fiziksel Bluetooth veya dokunmatik oyun kolunu izinli bir oturuma gönderip AfuDesk v1.4.0 için doğrulanabilir yayın hazırlamak.

**Mimari:** Dart, normalize edilmiş kol durumunu platform kanalı/katman üzerinden alır; Rust köprüsü durum mesajını izleyici kuyruğuna yollar, sıra numarasını Rust artırır ve mevcut datagram yolunu kullanır. Dokunmatik yüzey çoklu pointer ile durum üretir, aynı oturumda fare çevirisini kapatır; titreşim telefon ve desteklenen Bluetooth koluna aktarılır. Bağlantı hataları kullanıcıya sade metin, ayrıntılar log olarak verilir.

**Teknoloji:** Flutter/Dart, Android Kotlin KeyEvent/MotionEvent, flutter_rust_bridge, Rust QUIC datagram, GitHub Actions, Windows paketleme.

**Kapsam:** Köprü ve eşleme; fiziksel Android kol; dokunmatik kol ve izin UX'i; bağlantı ver/hata metinleri; testler/görev kapısı; EN+TR README ve notlar; CI, Android paketi, checksum ve v1.4.0 release.

**Kısıtlar:** NAT delme/DHT/relay yok; force push yok; başka depo yok; yeni pencere yok; teknik kullanıcı ayarı yok; başparmak alanları en az 56dp.

## Görevler

- [ ] Mevcut Rust datagram yoluna `izleyici_kol` ekle; sıra numarasını Rust atar ve masaüstü gilrs akışına dokunma. Köprü ve uçtan uca testini yaz.
- [ ] Saf Dart eşleme (XInput tuş maskesi, eksen/tetik aralıkları) ve değişiklikte en çok 250Hz gönderim durumunu yaz/test et.
- [ ] Android MainActivity platform kanalında KeyEvent/MotionEvent durumunu normalize edip Flutter'a ilet; titreşim geri dönüşünü bağla.
- [ ] Oturum araç çubuğuna izinli kol düğmesi ve yatay yarı saydam 2 çubuk/D-pad/A-B-X-Y/LB-RB/LT-RT/Start/Back katmanı ekle; çoklu dokunma, fare girdi engeli ve izin yok UX'i test et.
- [ ] `baglan` ve Flutter hata metnini sadeleştir; bağlan ekranı uyarı metnini ve regresyon testlerini ekle.
- [ ] `OYUN_MODU.telefon_kol` kapısını ekle; Python doğrulayıcıyı çalıştır. İstenen Rust/Flutter testlerini, analyzer ve release testlerini çalıştır.
- [ ] README EN+TR telefon testi, sürüm notu EN sonra TR; FRB codegen ve paketleme/Android artifact SHA doğrulaması.
- [ ] Branch/CI durumu uygunsa Türkçe commit, main birleştirme, push, CI+Android başarı kontrolü ve v1.4.0 release.

## İnceleme odakları

- İzleyici izni verilmeden hiçbir kol verisi gönderilmemesi; izin ekranına bağlı test.
- Android eksen tetik varyantları ve XInput sınırları; saf eşleme testleri.
- Eşzamanlı pointer bırakma/iptal; katman durum testi.
- Bağlantı hata zincirinden teknik ayrıntı sızmaması; Rust+Flutter testleri.
- Remote main 1.2.0 gösteriyor; 1.3.0'ın gerçekten mevcut olduğu yayın öncesi doğrulanmalı.
