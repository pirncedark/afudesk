# Otomatik güncelleme

Ana ekran açılışında, uygulamaya dönüldüğünde ve altı saatte bir GitHub'ın son kararlı yayını kontrol edilir. Yeni sürüm ve bu cihaza ait paket ile SHA-256 dosyası birlikte bulunursa Güncelle kartı gösterilir. İnternet veya yayın hatası ana kullanımı etkilemez.

- Windows: ZIP indirilir, boyutu ve SHA-256 özeti doğrulanır, hedef yazma izni ve paket kontrol edilir. Uygulama kapanır; yardımcı işlem eski dosyaları yedekler, yenilerini kopyalar ve AfuDesk'i açar. Kopyalama hatasında yedekler geri yüklenir. Yazma izni olmayan klasörde uygulama kapanmaz; güncelleme sayfası açılır ve yeni sürümü oradan indirme mesajı gösterilir.
- Android: APK uygulamanın özel önbelleğine indirilir ve doğrulanır. Kurulum izni yoksa sistem izin sayfası açılır, izin dönüşünde kurulum otomatik başlar. Android'in son kurulum onayını kullanıcı verir. İmza ve sürüm kodu denetimini Android yapar.

## Yayın sözleşmesi

"Sonra" seçilen sürümü yalnız bu uygulama açılışı boyunca gizler. Bağlantıda sürüm uyuşmazlığı algılanırsa güncelleme hemen kontrol edilir.

`pubspec.yaml` sürümü etiketle aynı olmalıdır. Her Android sürümünün build numarası artırılmalıdır. Release, CI artifact'larından Claude tarafından elle yayınlanır. Şu dört dosya birlikte yayınlanır:

- `AfuDesk-windows-x64.zip` (kökte `afudesk.exe`, `afudesk_kopru.dll`, `data/`; üst klasör yok)
- `AfuDesk-windows-x64.zip.sha256`
- `AfuDesk-android.apk`
- `AfuDesk-android.apk.sha256`

Release için `ANDROID_KEYSTORE_B64` ve `ANDROID_KEYSTORE_PASSWORD` zorunludur; anahtar önceki kurulumlarla aynı olmalıdır. Debug imzalı APK release olarak yayınlanmaz. Daha önce farklı anahtarla yüklenmiş APK yerinde güncellenemez.

Güncelleme özelliğini içermeyen eski sürümler bu kartı gösteremez; özelliği içeren ilk sürüm bir kez mevcut indirme yöntemiyle kurulmalıdır.

## Doğrulama

`flutter analyze` ve `flutter test` standart kontrollerdir. SDK yazma izni olmayan ortamda yayın seçimi için `dart tool/dogrula_guncelleme.dart` çalıştırılabilir. Windows dosya kurulumu için `powershell -File tool/test_windows_update.ps1` başarılı kopyalama, kısmi hata sonrası geri alma ve eksik paketin reddini sınar; uygulama açma ve hata penceresi testte kayıtla değiştirilir. Gerçek Windows kurulum/geri dönüşü, APK izin dönüşü ve aynı imzalı APK üzerine kurulum ayrıca cihazda denenmelidir.
