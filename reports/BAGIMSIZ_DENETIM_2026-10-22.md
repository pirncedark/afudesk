# AfuDesk — Bağımsız denetim ve düzeltme

Tarih: 22 Ekim 2026
Başlangıç: 97a2730 (release/v1.10.3)

## Doğrulanan kusurlar ve düzeltmeler

- Windows WGC CaptureControl otomatik durmuyor. Yakalayıcı bırakılırken stop çağrısı eklendi. İlk kare alınamadığında fallback yolu da temizlenir.
- Host oturumu erken döndüğünde/iptal edildiğinde yakalama durdurma bayrağı artık yaşam döngüsü korumasıyla ayarlanır.
- Pencere açma isteğinde telefonun gerçek width/height ölçüleri kullanılır; zorunlu yön değişimi ayrıca test edilir.
- Güncelleme kontrolü yalnız açılış/24 saat ile sınırlıydı. Uygulamaya dönüşte 5 dakikalık aralıkla tekrar kontrol edilir; ağ hatasından sonra tekrar denenir.
- Google hesap ekranında küçük ayarlar rozeti yerine güncelleme bulunduğunda görünür Güncelle düğmesi gösterilir.

## Mevcut özelliklerin durumu

- Pencereler seçimi v1.10.3 kaynaklarında vardır; terminal varken Diğer pencereler ile erişilir.
- Telefonun IME/dikte metnini gönderen hat vardır; gerçek Gboard/Samsung dikte testi yapılmadı.
- Bas-Konuş Space down/up gönderir ve iptal/kapanışta bırakır.
- Telefon mikrofonunun sesini uzak bilgisayara taşıyan bir hat yoktur. Space olayları ses aktarımı değildir; bu özellik tamamlanmış sayılmaz.
- Aynı Google hesabı için onaysız erişim kodu vardır; canlı Google hesabıyla 10 bağlantı doğrulanmadı.

## Test kapsamı ve sınırlamalar

- Flutter tüm otomatik testleri ve analiz bağımsız çalıştırıldı: 187/187 test PASS, flutter analyze No issues found.
- Rust CI ile aynı şekilde gercek_ testleri hariç çalıştırıldı; 134 birim + 21 entegrasyon testi geçti (155 PASS). Yeni 10 aynı host bağlantı testi 42.53 saniyede PASS.
- Gerçek ekran testli lib çalışmasında 138 test geçti, iki test 0x80070006 geçersiz tutamaç hatasıyla kaldı. Bu sonuç gerçek masaüstü doğrulaması değildir.
- Yeni pencere yönü ve tekrar geçiş testleri sahte motor kullanır.
- Gerçek telefona bağlı ADB cihazı bulunmadı; cihaz testi yapılamadı.
- 5 saatlik kaynak ölçümü yapılmadı. Sarı çerçeve ve fare donması gerçek sistemde tekrar ölçülmelidir.
- Sunucuda release bulunması, telefonda güncellemenin kurulduğu anlamına gelmez.

## Kalan işler

1. Gerçek telefonda APK güncellemesi, Google oturumu, pencere dokunması ve klavye diktesi.
2. Gerçek Windows masaüstünde 10 yakalama/kapatma ve 5 saat kaynak ölçümü.
3. Telefon mikrofonundan uzak uygulamaya ses taşıma tasarımı ve uygulaması.
4. Host olay akışı beklenmedik kapanınca arka plan yeniden başlatma davranışının ayrıca hatayı üreten testle incelenmesi.
