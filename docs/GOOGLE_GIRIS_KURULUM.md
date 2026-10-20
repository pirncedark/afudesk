# AfuDesk — Google girişi ve Windows başlangıç hazırlığı

## Durum ve kapsam

EKİP-3 platform teslimidir: Android yapılandırması + Keystore, Windows DPAPI,
isteğe bağlı oturum açılış görevi betikleri ve CI kontrolleri hazırlandı.
Bu dal tek başına Google girişini veya gözetimsiz bağlantıyı etkinleştirmez.
`app/lib`, `app/test` ve `rust/src` değiştirilmedi. EKİP-1 kimlik doğrulamasını,
EKİP-2 giriş ekranını ve aşağıdaki platform kanalı çağrılarını bağlamalıdır.

Uygulama kullanıcısı için hedef: **Google ile giriş yap → bilgisayarını seç → bağlan.**
Aşağıdaki OAuth kurulumu uygulamayı derleyen/yayımlayan kişi içindir; her son
kullanıcıdan Cloud Console ayarı istenmez.

Uygulama sırası: mevcut imzayı belirle → OAuth istemcilerini tanımla → platform
saklamasını bağla → CI derlemeleri ve gerçek cihaz giriş/çıkış testlerini doğrula.
Arka plan görevi yalnız ayrı ve açık kullanıcı eylemiyle kurulmalıdır.

## Bir kez yapılacak ücretsiz OAuth kurulumu

1. [Google Cloud Console](https://console.cloud.google.com/)'da mevcut projenin
   Google Auth Platform bölümünü aç. Yoksa ücretsiz proje oluştur; bu işlem için
   faturalandırma, Firebase, ücretli API veya yeni sunucu gerekmez.
2. Branding/Audience bölümünde uygulama adını ve destek e-postasını tanımla.
   Deneme aşamasında kullanacağın hesabı test kullanıcısı ekle. Yalnız
   `openid email profile` kapsamlarını kullan; Drive/Gmail izni isteme.
3. Clients bölümünde **Android** istemcisi oluştur. Paket adı
   `com.afu.afudesk`; SHA-1 aşağıdaki gerçek imza anahtarından alınır.
   Debug ve yayın anahtarı farklıysa her SHA-1 için ayrı Android istemcisi ekle.
4. Aynı projede **Web application** istemcisi oluştur. Bu, Android
   `serverClientId` / Credential Manager `setServerClientId` için gereken ID'dir;
   Android türündeki client ID bu alana yazılmaz. Sadece yerel giriş için Web
   istemcisinin client secret'ı uygulamaya konulmaz.
5. Aynı projede **Desktop app** istemcisi oluştur. Windows bu ID ile sistem
   tarayıcısı ve loopback dönüşünü kullanır; Web istemcisini masaüstüne verme.
6. Web ID'yi yerel Gradle ayarına, Desktop ID'yi EKİP-1/2'nin derleme ayarına
   aktar. Gerçek ID, JSON, parola veya anahtar dosyası commit edilmez.

### Android SHA-1: bu deponun imza seçimi

`app/android/app/build.gradle.kts`, `app/android/key.properties` varsa
`storeFile`, `storePassword`, `keyAlias`, `keyPassword` alanlarını okuyup `afu`
imzasını kullanır; dosya yoksa **release dahi debug anahtarıyla imzalanır**.
CI `.github/workflows/android.yml`, `ANDROID_KEYSTORE_B64` varsa `afudesk.p12`
ve alias `afudesk` ile aynı dosyayı oluşturur. Bu nedenle rastgele yeni anahtar
üretme; dağıtılan APK'yı imzalayan anahtarın SHA-1'ini kullan.

SDK kurulu geliştirici makinesinde daha sonra çalıştırılabilecek komutlar
(bu görevde çalıştırılmadı):

```powershell
# app/android dizininde; Flutter SDK ve local.properties gerekir:
.\gradlew.bat :app:signingReport

# Alternatif: key.properties içindeki gerçek storeFile ve keyAlias değerleri:
keytool -list -v -keystore 'C:\anahtarlar\afudesk.p12' -alias afudesk

# Debug anahtarı ancak daha önce bir debug derlemesi üretildiyse vardır:
keytool -list -v -keystore "$env:USERPROFILE\.android\debug.keystore" -alias androiddebugkey
```

Parolayı etkileşimli gir; komut satırına veya loga koyma. Debug anahtarının
standart parolası `android`'dir. `signingReport` çıktısındaki **SHA1** kullanılır.
Play App Signing kullanılırsa upload anahtarı yerine dağıtım imzasının SHA-1'i
de Android istemcisine kaydedilmelidir.

### Client ID'lerin yeri

Kullanıcıya ait, depoya girmeyen `%USERPROFILE%\.gradle\gradle.properties`:

```properties
AFUDESK_GOOGLE_WEB_CLIENT_ID=WEB_CLIENT_ID.apps.googleusercontent.com
```

Aynı isimli ortam değişkeni de desteklenir; Gradle property önceliklidir.
Boş varsayılan derlemeyi bozmaz, ancak gerçek Google girişi yapılamaz.
Android CI, isteğe bağlı repository variable `AFUDESK_GOOGLE_WEB_CLIENT_ID`
değerini okur. Client ID bir parola değildir; yine de gerçek yapılandırma
depoda tutulmaz. `key.properties`, `*.p12`, `*.jks`, `*.keystore` ve Android
`google-services.json` dosyaları ignore edilir.

Windows için önerilen derleme parametresi
`--dart-define=AFUDESK_GOOGLE_DESKTOP_CLIENT_ID=...` şeklindedir; bu dal Dart
okuyucusunu eklemez. EKİP-2 kendi ayar adıyla bunu eşlemelidir. `.env` otomatik
okunmaz; gerekiyorsa uygulama dışındaki yerel dosyadan derleme ortamına aktar.
Desktop istemcisinin indirilen JSON'unu ve client secret'ını commit etme.
Dağıtılmış masaüstü uygulaması güvenilir bir client secret saklayamaz;
sağlayıcının kod değişiminde istediği masaüstü metadata'sı kimlik kanıtı değildir.

## EKİP-1 / EKİP-2 entegrasyon sözleşmesi

### Android Google ayarı

`MethodChannel('afudesk/google_config').invokeMapMethod('read')` şu alanı verir:
`webClientId: String`. Bunu `google_sign_in` başlatıcısının `serverClientId`
alanına veya Credential Manager `setServerClientId` alanına aktar.
Kaynak adı `R.string.afudesk_google_web_client_id`'dir. Boşsa kullanıcıya
tek cümlelik giriş hatası göster; geliştirici tanısı için token içermeyen bir kod kullan.
Mevcut INTERNET izni yeterlidir; Android'e loopback/custom-scheme intent filtresi
eklenmedi. Firebase/Google Services Gradle eklentisi gerekmiyor.
Bu dal giriş SDK'sı eklemez: `google_sign_in` seçilirse pubspec bağımlılığı
ve nonce destekli başlatma EKİP-2 kapsamındadır. Credential Manager doğrudan
seçilirse AndroidX Credentials + Google ID bağımlılıkları ve giriş çağrısı ayrıca gerekir.

### İki platformda şifreli saklama

`MethodChannel('afudesk/secure_storage')`:

| Metot | Argüman | Sonuç |
|---|---|---|
| `write` | Doğrudan UTF-8 JSON **String**, 1–65536 bayt | `null` |
| `read` | Yok | JSON String; kayıt yoksa `null` |
| `clear` | Yok | `null`; tekrar çağrılabilir |

Tek bundle içinde oturum sırrını, eşleşme kaydını ve gerekiyorsa cihaz özel
anahtarını sakla. Rust'ın ayrı dosyaya açık metin yazması bu kanalla otomatik
engellenmez; EKİP-1 eski kalıcı yazma yolunu kapatıp bu köprüye bağlamalıdır.
Kanal kimlik doğrulamaz; kayıt ancak doğrulanmış Google kimliğinden üretilmelidir.
Hatalar `secure_storage` koduyla döner, token/sır/log/ayrıntılı exception içermez.

Android: AndroidKeyStore AES-256-GCM, her yazmada yeni IV, sürüm + IV +
şifreli içerik, sabit AAD ve uygulamaya özel SharedPreferences. Anahtar dışarı
çıkarılmaz. `clear` anahtarı da siler. Anahtar kayıp/şifreli kayıt bozuksa
sessizce yeni oturum oluşturulmaz; tekrar giriş gerekir. Yedekleme kapalıdır;
ayrıca cloud/device-transfer kuralları sır kaydını dışlar. Android gerçek cihazda
yeniden açma, bozuk kayıt, yedekten dönüş ve çıkış testleri ayrıca yapılmalıdır.

Windows: yalnız mevcut kullanıcı kapsamında DPAPI, `CRYPTPROTECT_UI_FORBIDDEN`;
`LOCAL_MACHINE` kullanılmaz. Dosya `%LOCALAPPDATA%\AfuDesk\google-session.dpapi`.
Yazma şifreli geçici dosyadan atomik değiştirmeyle yapılır; `clear` geçici
dosyayı da kaldırır. Klasör kullanıcı profilinin ACL'sini devralır. Aynı Windows
kullanıcısı altında çalışan zararlı yazılıma karşı ek bir güven sınırı değildir;
başka kullanıcı/servis hesabı bu kaydı kullanmamalıdır. Silme fiziksel diskten
güvenli yok etme veya eski yedekleri geçersizleştirme garantisi vermez.

Çıkışta önce bağlantıları kapat, hesap cihaz kaydını kaldır/geçersizleştir,
`clear` sonucunu bekle, bellekteki token/cihaz listesini temizle. `clear`
başarısızsa kullanıcıya çıkış tamamlandı deme. Android SDK credential state'ini
de temizle. Uygulama görünür uzak bağlantı göstergesini ve tek dokunuşla kesmeyi
korumalıdır; bu platform yardımcıları bunların yerine geçmez.

### Windows OAuth dönüşü (çekirdek / Dart sorumluluğu)

Mevcut runner komut satırını Dart'a iletir; özel URI kaydı gerekmez. Native
OAuth sunucusu eklenmedi; EKİP-1/2 tek bir loopback akışının sahibi olmalıdır:

1. Önce yalnız `127.0.0.1` üzerinde port `0` ile dinle; atanan rastgele portu al.
2. Kriptografik rastgele `state`, OIDC `nonce`, PKCE verifier oluştur; challenge
   S256 olsun. Sistem tarayıcısını aç; gömülü WebView kullanma.
3. `http://127.0.0.1:<atanan-port>/oauth/callback` dönüşünü yalnız beklenen
   path, HTTP metodu ve state için kabul et. `0.0.0.0` dinleme; güvenlik duvarı
   istisnası veya yönetici izni isteme. İptal/zaman aşımında dinleyiciyi kapat.
4. Kodu aynı redirect URI ve verifier ile HTTPS üzerinden bir kez değiştir.
   Beklenmeyen isteklerde bekleyen gerçek akışı tüketme; tokenı tarayıcıya/loga yazma.
5. ID token imzasını Google JWKS ile doğrula; `iss`, `aud` (Android Web veya
   Desktop için açık allowlist), `exp`, `nonce`, gerektiğinde `azp` kontrol et.
   E-posta eşitliği yerine `sub` kullan. Onaysız erişim için ayrıca cihaz anahtarı
   bağını ve yeniden oynatmayı önleyen oturum kanıtını doğrula.

## PC arka plan erişimi: gerçek sınırlar

| Seçenek | Sağladığı | Sağlamadığı / risk |
|---|---|---|
| Kullanıcı oturumunda zamanlanmış görev (önerilen ilk aşama) | Kullanıcı girişinde yönetici/parola gerektirmeden uygulamayı açar | Oturum kapalıyken çalışmaz; uygulama kapanınca erişim biter; kilit/UAC ekranı garantisi yok |
| Windows servisi + kullanıcı oturumunda ajan | Açılışta servis çalışabilir; oturum ajanının yaşam döngüsü yönetilebilir | Yönetici kurulumu, Session 0 izolasyonu, güvenli IPC/oturum seçimi ve ayrı yakalama tasarımı gerekir |

Bu sürümde `main.cpp` içindeki `SetQuitOnClose(true)` pencere kapanınca süreci
sonlandırır. Sırf başlangıç göreviyle **uygulama kapalıyken erişim** sağlanamaz.
Gerçek arka plan host/tray yaşam döngüsü EKİP-1/2'de ayrıca uygulanmalıdır;
hazırlanan betik desteklenmeyen `--headless` parametresi göndermez.
Kilit ekranı, UAC secure desktop ve kullanıcı girişi öncesi görüntü/girdi
yakalama ayrıca tasarlanıp test edilmelidir. Güvenli masaüstünü veya UAC'yi
kapatmak çözüm değildir. Windows servisi normal kullanıcı oturumunun DPAPI
kaydını doğrudan kullanamaz; LocalSystem'e token kopyalanmamalıdır.

`scripts/arka_plan_kur.ps1` yalnız çağıran kullanıcının SID'sine özel
`AfuDesk-Background-<SID>` adlı giriş görevini hazırlar. Tam exe yolu ve çalışma
dizini ayrı alanlardır, parola ve komut satırı sırrı yoktur. Mevcut görevi
günceller; uygulamayı hemen çalıştırmaz. `arka_plan_kaldir.ps1` yalnız aynı
SID'nin görevini kaldırır; çalışan uygulamayı ve hesap kaydını silmez.
Her iki dosya `-WhatIf` destekler. Kurumsal politika görevi reddederse
yöneticiye yükselmeyi otomatik deneme. Taşınan uygulamanın yolu yeniden ayarlanmalıdır.

İleride kullanıcı açıkça isterse çalıştırabileceği örnekler (bu görevde çalıştırılmadı):

```powershell
.\scripts\arka_plan_kur.ps1 -ExePath 'C:\Apps\AfuDesk\afudesk.exe' -WhatIf
.\scripts\arka_plan_kur.ps1 -ExePath 'C:\Apps\AfuDesk\afudesk.exe'
.\scripts\arka_plan_kaldir.ps1
```

PIN kararı ana ekipte açık: varsayılan açık önerilir; 6+ hane, tuzlu/yavaş hash,
deneme sınırlaması, düz metin PIN kaydı yok. Bu dal PIN ekranı/kararı eklemez.

## Doğrulama ve kalanlar

Mevcut CI `cargo test --release -- --skip gercek_`, `flutter analyze`,
`flutter test`, Windows release derlemesini korur. Eklenenler: PowerShell
dosyalarını yalnız `Parser::ParseFile` ile ayrıştırma ve bağımsız native DPAPI
testini MSVC ile derleyip çalıştırma. Test kendi geçici dizinini kullanır;
gerçek oturum kaydına dokunmaz. Android akışı artık PR'da da APK derler ve
Flutter testlerini çalıştırır. Boş OAuth ID ile derleme testi giriş testi sayılmaz.

Yerel kontrol (21 Ekim 2026): iki PowerShell dosyası 0 ayrıştırma hatası;
çalıştırılmadılar. Flutter, MSVC (`cl`), CMake ve Kotlin compiler PATH'te yok;
`app/android/local.properties` yok, `ANDROID_HOME`/`ANDROID_SDK_ROOT` tanımsız.
Gradle launcher var ama Flutter/SDK olmadan proje derlenemez; betik yasağı
nedeniyle launcher/kurulum betikleri de çalıştırılmadı. Native DPAPI testi ve
Android/Windows derlemeleri yerelde çalıştırılamadı; CI sonucu henüz yok.

Birleştirmeden sonra gerçek Android ve Windows'ta aynı hesapla giriş, yeniden
açma, çıkışta silme, bozuk kayıt reddi ve farklı hesapta onay akışı denenmeli.
Bu kontrol listesi tamamlanmadan ürünün gözetimsiz erişimi tamamlandı denmemeli.

**Kullanıcının tek adımı:** Yayıncı olarak ücretsiz OAuth istemcilerini bir kez
oluşturup ID'leri yukarıdaki yerel/CI ayarlarına koy; gerçek değerleri sohbete gönderme.

## Resmî kaynaklar

- [Google masaüstü OAuth, loopback ve PKCE](https://developers.google.com/identity/protocols/oauth2/native-app)
- [Android Credential Manager ile Google girişi](https://developer.android.com/identity/sign-in/credential-manager-siwg)
- [Windows kullanıcı kapsamlı DPAPI](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata)
