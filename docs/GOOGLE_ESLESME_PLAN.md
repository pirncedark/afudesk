# Google hesabıyla eşleşme uygulama planı

Kaynak: `GOREV_afudesk_google_eslesme.md` (21 Ekim 2026), EK dahil.
Durum: uygulama sürüyor; doğrulama ve eksikler `reports/GOOGLE_ESLESME_SONUC.md` içinde tutulacak.

## Amaç ve mimari

Windows ve Android aynı Google `sub` hesabına bağlı iroh anahtarlarıyla birbirini tanır.
Mevcut kod/parola ve kayıtlı cihaz bağlantıları korunur. Google bağlantısı ayrı,
başarısızlıkta kapalı kalan bir yetkilendirme yoludur; eski kayıt tek başına onaysız erişim vermez.

İnceleme: `rust/src/kimlik.rs`, `guvenilen.rs`, `host.rs`, `izleyici.rs`,
`protokol.rs`, `app/kopru/src/api/afudesk.rs`, `app/lib/main.dart`,
`app/lib/sayfalar/ana.dart`. Kayıtlı cihaz dalı salt okunur incelendi:
`ozellik/kayitli-cihazlar` içindeki iroh kimliği ve jeton sistemi zaten main'de;
main ayrıca cihaz etiketlerini koruyor. Eski dalın dosyaları kopyalanmayacak.

**Önemli bulgu:** `kapi/gorevler.json` test kapısının girdisidir; bir HTTP hesap
servisi değildir. n0 relay yalnız şifreli paket taşır, hesap dizini sağlamaz.
Yeni ücretli servis veya bulut hesabı oluşturulmaz. Cihaz dizininin nerede
çalıştırılacağı kullanıcıya ait dağıtım kararıdır; kod hazırlanabilir, canlıya kurulmaz.

### Giriş ve kimlik

- Windows: sistem tarayıcısı, loopback callback, OAuth authorization code + PKCE
  S256 + rastgele state. Google nonce, kalıcı host/izleyici açık anahtarlarına bağlanır.
- Android: Credential Manager, aynı nonce ile Google ID token; platform kanalı.
- Rust ve dizin: Google JWKS sabit HTTPS adresinden alınır, yalnız RS256;
  imza, kid, aud (izinli proje istemcileri), iss, iat, exp ve nonce doğrulanır.
  Kimlik `sub` değeridir; e-posta yetki vermez. Bozuk/geçersiz token reddedilir.
- Google istemci kimlikleri dağıtım yapılandırmasıdır; boş/yer tutucu yapılandırma
  hiçbir zaman gerçek giriş veya onaysız erişim sağlamaz. Sırlar/loglar git'e girmez.

### Cihaz dizini ve bağlantı

Ücretsiz kendi sunucusunda çalıştırılabilecek dizin katmanı: hesaba bağlı iroh
anahtarları, görünen ad ve ulaşım adresleri. Google token'ı listeye yayımlanmaz.
Dizin kaydı Google nonce'ındaki anahtarlara ve anahtar sahipliği kanıtına bağlı
olmalıdır. Kaldırma işlemi eski token ile tekrar kayıt yapılmasını da engellemelidir.
Host her Google bağlantısında güncel kayıt/iptal durumunu kontrol etmelidir;
dizin erişilemiyorsa onaysız erişim verilmez. Bağlantı uç kimliği iroh el sıkışmasından alınır.

Onaysız karar yalnız `rust/src/host.rs` içinde, ekran/girdi oluşturulmadan önce:
iki doğrulanmış aynı sub, karşı uç anahtarı hesaba kayıtlı, kayıt iptal edilmemiş,
anahtar açık. Farklı hesap/kapalı anahtar mevcut onaylı yola gider;
bozuk veya süresi geçmiş Google kanıtı reddedilir. Oturum sırasında kökte görünür
"AfuDesk ile uzaktan bağlı: <cihaz adı>" ve tek dokunuşla kes düğmesi gerekir.
Çıkış ve cihaz kaldırma, etkin Google oturumunu da kesmelidir.

### Kalıcı güvenlik

Windows DPAPI, Android Keystore/AES-GCM. Hesap/cihaz kaydı diskte açık JSON
olamaz; çözme hatası boş hesaba dönüşür, yetki vermez. Çıkışta hesap kaydı,
eşleşmeler ve hatırlanan PIN yetkileri silinir. Eski kodla eşleşme Google hesabı sayılmaz.

### EK: sürekli erişim ve PIN

- Kullanıcı görevi zamanlayıcı: yalnız kullanıcı oturumu açıkken güvenilir;
  uygulama kapalıyken ayrı host sürecini başlatabilir, oturum kapalıyken çözüm değildir.
- Windows servisi: açılıştan ve kullanıcı girişinden önce dinleyebilir; Session 0
  ekran/girdi yakalayamaz. Etkileşimli oturum ajanı, sınırlı IPC/ACL, kilit ekranı
  ve UAC secure desktop tasarımı gerekir. SYSTEM yetkisi saldırı yüzeyini büyütür.
- Bu tur servis/autostart kurulmayacak. Normal kullanıcı host'u, oturum kapalıyken
  erişim sağlıyormuş gibi gösterilmeyecek. Servis/ajan protokolü güvenlik sınırıdır.
- **Öneri:** PIN varsayılan açık, en az 6 rakam; Argon2id + rastgele tuz;
  yalnız PHC hash saklanır. Bağlantıda deneme sınırı ve süreli kilitleme gerekir.
  "Bu cihazda hatırla" ham PIN saklamaz; cihaz anahtarına bağlı, iptal edilebilir
  yetki saklar. Varsayılanı ve hatırlama süresi kullanıcı kararı olarak açık tutulur.

## Tehdit modeli

1. Çalınan Google token: nonce anahtara bağlıdır; başka iroh anahtarı kullanamaz.
   Ele geçirilmiş yerel oturum/anahtarın etkisini iptal ve kısa token ömrü sınırlar.
2. Sahte cihaz/dizin: sub metnine güvenilmez; imzalı token + uç kimliği eşleşir;
   dizin cevabı tek başına onaysız kabul sebebi olamaz.
3. Tekrar oynatma: rastgele OAuth state/PKCE; bağlantıda taze challenge,
   hedef anahtarı ve oturum kapsamı. İptal edilmiş eski kayıt diriltilemez.
4. Dizin kesintisi/bozuk vault/yanlış yapılandırma: erişim kapalı kalır.
5. Kullanıcı çıkışı/ayar değişimi/cihaz kaldırma: yeniden bağlantı jetonu eski
   Google yetkisini devam ettiremez. Etkin oturumun kesilmesi gerekir.

## İş paketleri ve gerçek dosya yolları

- [ ] 1. `rust/src/google.rs`: JWKS/RS256 doğrulama, özel doğrulanmış kimlik tipi,
  anahtar bağlama ve onaysız karar; `rust/tests/google_guvenlik.rs` negatif testler.
- [ ] 2. `rust/src/google_depo.rs`: DPAPI ve Android güvenli depo sınırı;
  `rust/src/google_pin.rs`: tuzlu PIN hash doğrulaması; `rust/src/lib.rs`,
  `rust/Cargo.toml`, `rust/Cargo.lock` bağımlılıklar.
- [ ] 3. Dizin, OAuth ve platform bağlantı kodu, host/izleyici entegrasyonu;
  `app/kopru/src/api/afudesk.rs` ve üretilen köprü dosyaları.
- [ ] 4. `app/lib/main.dart`, `app/lib/sayfalar/ana.dart`, yeni hesap ekranı;
  `app/android/app/src/main/kotlin/com/afu/afudesk/` giriş/Keystore;
  `app/android/app/build.gradle.kts` Credential Manager bağımlılıkları.
- [ ] 5. Güvenlik regresyonları, Rust test/clippy, Flutter analiz/test,
  mümkünse Android derleme; `reports/GOOGLE_ESLESME_SONUC.md` gerçek çıktılar.

Her pakette önce negatif test, ardından uygulama ve doğrulama. Eksik altyapı
nedeniyle tamamlanmayan paketler tamamlandı diye işaretlenmez.

## Kullanıcının yapacağı tek OAuth adımı

Google Cloud Console'da ücretsiz OAuth projesi/istemcilerini oluştur:
Windows için Desktop; Android uygulama paket adı ve imza SHA-1'i için Android,
Credential Manager serverClientId için aynı projede Web istemcisi.
Gerçek değerler bu görevde istenmez; dağıtıcı bunları derleme yapılandırmasına koyar.
Bu işlem, bir hesap/cihaz dizini barındırma hizmeti kurmaz.

## Kullanıcı kararı gereken açık sorular

1. Ücretsiz hesap/cihaz dizini mevcut hangi sunucuda barındırılacak?
   Alternatif: ilk eşleştirmesi kodla yapılan tamamen eşler arası model;
   bunun farklı ağlarda ilk cihazı kendiliğinden listeleyemediği kabul edilmeli.
2. PIN varsayılan açık önerisi kabul mü; hatırlama süresi ne olmalı?
3. Giriş öncesi/kilit ekranı erişimi için Windows servis + oturum ajanı mı,
   yalnız kullanıcı oturumu açıkken çalışan arka plan host'u mu?

## Tur 3 devam kaydı (21 Ekim)

Önce `git status --short`, `git diff --stat`, `git diff` okundu. Önceki
iki turun token/JWKS, DPAPI, PIN, loopback OAuth ve Android Credential Manager /
Keystore dosyaları korundu. Başlangıç HEAD `e7e0a07`; önceki turlarda commit yok.

Bu tur eklenen `rust/src/google_dizin.rs`, dağıtım bağımsız bellek içi dizin
çekirdeğidir; çalışan HTTP/iroh hesap servisi değildir. Kayıt Google tarafından
imzalanmış tek kullanımlık nonce'a, host ve izleyici anahtarlarının ayrı Ed25519
imzalarına ve cihaz adına bağlıdır. İşlem 300 saniye geçerlidir; hatalı denemede
de tüketilir. Kaldırma bekleyen işlemleri iptal eder; tüketilmiş token yeniden
kayıt açamaz. Anahtar başka hesap/cihaz tarafından sessizce sahiplenilemez.
İki ayrı cihazın host/izleyici anahtarları aynı güncel hesap dizininde aranır.
`Giris::dizin_nonce` dizin nonce'unu değiştirmeden Google'a aktarır.

**Entegrasyondan önce zorunlu kalanlar:**

1. Kalıcı, işlem bütünlüğü olan dizin adaptörü ve kimliği doğrulanmış ağ
   yönetim katmanı. `liste`/`kaldir` metotları ham istemci `sub` alanına açılmaz.
   Dizin yeniden başlatılınca bellek içi kayıtlar kaybolur; erişim kapalı kalır.
2. `google.rs::karar` önceki turun bağımsız birimidir, gerçek host tarafından
   çağrılmıyor. Sabit anahtar nonce'u bekler; rastgele OAuth kayıt nonce'u bu
   metoda doğrudan verilmez. Kayıt kanıtı ile taze bağlantı challenge'ı ayrı
   tasarlanıp host/izleyici protokolüne bağlanmalı; tekrar oynatma ve iptal testi
   gerçek uçtan uca akışta yapılmalı. `kayitli` bool'u istemciden alınmamalı.
3. Flutter/Rust köprü, giriş ekranı, Bilgisayarlarım, çıkış/kaldırma, ayar,
   görünür oturum göstergesi ve kes düğmesi. Bugün UI Google girişi sunmuyor.
4. PIN kalıcı PHC yükleme, deneme sınırı/kilitleme ve hatırlama yetkisi.
5. Kurulum yapmadan Windows host/servis + oturum ajanı kodu; Session 0 ve
   kilit ekranı güvenlik sınırları. Bu özellik henüz hazırlanmadı.

Bu kayıt tamamlanma beyanı değildir. Sonraki tur bu listeden devam etmeli;
mevcut güvenlik ilkellerini yeniden yazmamalı.

## Resmî teknik kaynaklar

- https://developers.google.com/identity/openid-connect/openid-connect
- https://developers.google.com/identity/protocols/oauth2/native-app
- https://developer.android.com/identity/sign-in/credential-manager-siwg

## Tur 4 devam kaydı (21 Ekim)

Status/diff ile önceki turlar kontrol edildi; var olan dosyalar yeniden yazılmadı.
Bu tur PIN güvenlik paketi ilerletildi (`rust/src/google_pin.rs`,
`rust/tests/google_pin_kasa.rs`, `rust/tests/google_depo_pin.rs`).

- `Pin::phc_yukle`: yalnız uygulamanın sabit maliyetli Argon2id/v19,
  m=19456/t=2/p=1 PHC kaydını yükler; bozuk/aşırı maliyetli parametreleri reddeder.
- `PinKasasi`: hesap ve host kimliğine bağlı Windows DPAPI kaydı; beş yanlış
  denemede 300 saniye host genelinde kilit. Farklı izleyici anahtarı veya yeniden
  başlatma bütçeyi sıfırlamaz. Saat geriye giderse erişim verilmez.
- Oku/değiştir/yaz işlemi ayrı `.lock` dosyasının işletim sistemi kilidi altında;
  aynı süreçte veya ayrı host süreçlerinde denemeler bütçeyi aşamaz. Kilit dosyası
  boş kalır; süreç kapanınca kilit serbest kalır. Asıl kayıt atomik yazılır;
  kalıcı yazma başarısızsa kabul sonucu dönmez.
- Hatırlama yalnız doğrulanmış izleyici anahtarına bağlı yerel yetkidir;
  ham PIN/bearer jetonu yoktur. Süre çağıran tarafından açıkça verilir, azami 30 gün;
  ürün varsayılanı seçilmedi. `unut` cihaz yetkisini, `cikis` tüm PIN kaydını siler.
- Bu API Google token/dizin/iroh kontrollerinden SONRA çağrılmalıdır. Anahtarlar
  ve saat uzak istemci alanlarından alınmaz. Hatırlama iptal edilmiş cihazı
  yeniden yetkilendirmez; her bağlantıda güncel dizin kontrolü yine zorunludur.
- Android Keystore içinde aynı deneme/hatırlama modeli henüz bağlanmadı.
  Android Rust DPAPI adaptörü desteklenmediği için açık metin yedeğe düşmez.

Test döngüsü: eksik API RED; ilk uygulamada PHC parametre sayısı kontrolü yanlış
olduğu için testler reddetti. Kütüphane kaynak kodu `ParamsString::len` değerinin
karakter sayısı olduğunu gösterdi; `iter().count()` ile düzeltildi.
Kanıtlar `reports/google-tur4-*.txt`.

**Tur 5 önceliği:** Tur 3 listesindeki gerçek host/izleyici challenge/protokol ve
iptal entegrasyonu; doğrulanmış kalıcı ağ dizini; Flutter köprü/ekranlar/gösterge;
Android PIN adaptörü; servis/oturum ajanı kodu. PIN kasası gerçek host'a henüz
bağlı değildir. Görev tamamlanmış değildir; çalışan Google uzak bağlantısı yoktur.
