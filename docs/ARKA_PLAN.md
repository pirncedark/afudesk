# AfuDesk arka plan erişimi

1. Windows uygulamasında bir kez Google ile giriş yap.
2. Ayarlar → **Bilgisayar açılınca hazır olsun** anahtarını aç (varsayılan kapalı).
3. Pencereyi X ile kapat; AfuDesk sistem tepsisinde dinlemeye devam eder.

Tepside **Aç** pencereyi gösterir. **Bağlantıları durdur** yerel erişimi ve
bağlantıları keser; yeniden başlatana kadar durur. **Çıkış** uygulamayı kapatır.
Bu iki işlem Google kasasını silmez. Hesaptan çıkış kasayı siler ve Google
erişimini kaldırır. Başlangıç anahtarını kapatmak otomatik açılmayı ve X ile
tepsiye gizlenmeyi kapatır; açık bağlantıyı kesmez.

`afudesk.exe --arka-plan` pencere göstermeden başlar. Tepsi oluşturulamazsa
uygulama görünür açılır. İkinci açılış yeni host oluşturmaz; normal açılış mevcut
pencereyi öne getirir. İlk giriş yapılmamışsa tepsiden Aç ile giriş yapılmalıdır.

Windows yenileme paketi `%LOCALAPPDATA%\AfuDesk\google-session.dpapi` içinde
CurrentUser DPAPI ile şifrelenir. Token tercih dosyası veya tanıya yazılmaz.
Açılışta sessiz yenileme yapılır; ağ yoksa kasa korunur ve 30 saniye sonra
yeniden denenir. Çevrimiçi kayıt 60 saniyede bir yenilenir. Geçersiz yenileme
belirteci için **Google ile yeniden giriş yap** gösterilir.

Başlangıç için yönetici gerektirmeyen
`HKCU\Software\Microsoft\Windows\CurrentVersion\Run\AfuDesk` seçildi:
görev zamanlayıcısına göre daha az bileşen içerir ve kullanıcı açmadan yazılmaz.
Değer `"<exe yolu>" --arka-plan` biçimindedir. Uygulama yeni konumundan ilk
açıldığında mevcut kayıt güncellenir; hiç açılmazsa eski yol kendiliğinden
bulunamaz. Eski `scripts/arka_plan_kur.ps1` görevini ayrıca kurmayın; önceden
kurulmuş görev uygulamadaki anahtarla kaldırılmaz, kendi kaldırma betiğine aittir.

Bu bir Windows hizmeti değildir: kullanıcı oturum açmadan çalışmaz. Bilgisayar
açık ve uyanık olmalıdır. Kilit/UAC güvenli masaüstü görüntüleme ve kontrolü
desteklenmez. Bilgisayarı uyandırmaz.

Google bağlantısı istemsiz koparsa uygulama 1, 2, 4 saniye bekleyerek üç yeni
bağlantı dener; ekranda tek satır durum gösterir. Kullanıcı bağlantıyı keserse
yeni deneme yapılmaz. Bağlantının kendi ağ zaman aşımı bu beklemelere eklenir.
`GercekHesapServisi.sonBaglanmaSuresi` ilk kabul olayına kadar geçen süreyi tutar;
ilk görüntüye kadar olan süreyi ölçmez.

Chrome Uzak Masaüstü'nü kaldırmadan önce yeniden başlatma sonrası, telefonla
aynı hesap üzerinden görüntü/kontrol ve ağ kesintisi sonrası geri dönüşü gerçek
cihazlarda doğrulayın. Kilit ekranı gerekiyorsa geçiş tamamlanmış sayılmaz.

## Doğrulama sınırı

Yalnız dosya düzenleme talimatı nedeniyle testler, uygulama, başlangıç kaydı ve
betikler çalıştırılmadı. Yerel ağ bağlantı süresi ölçülmedi.
Kalan kontroller `ARKA_PLAN_SONUC.md` içindedir.
