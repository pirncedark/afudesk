# Google girişi ve 30 günlük cihaz güveni

EK4, eski EK3'ün yerini alır. Windows ve Linux aynı Rust hesap köprüsünü kullanır.

1. Google OAuth, mevcut desktop istemcisiyle sistem tarayıcısı, loopback callback, tek kullanımlık state/nonce ve PKCE kullanır. JWT imzası, issuer, audience, zaman ve nonce doğrulaması korunur. Üretimde test HTTP uçları açılamaz.
2. AfuDesk ayrı bir cihaz kaydını kalıcı host iroh anahtarıyla imzalar. Kayıt host/viewer anahtarlarını, Google sub/istemci kimliğini, rastgele oturum kimliğini ve zamanı bağlar. Güven tam 2.592.000 saniyedir; OAuth yenilemesi bu süreyi uzatmaz. Son kullanım zamanı geriye alınamaz.
3. Yeniden açılışta şifreli bundle, bağımsız yerel imzalı çapa ve kalıcı cihaz anahtarları birlikte kontrol edilir. Eski imzasız masaüstü kaydı, eksik/silinmiş çapa, değiştirilmiş hesap/anahtar/istemci veya süre aşımı yeniden giriş ister.
4. Kısa ömürlü Google access/ID tokenları yenilenir. Google yanıtında yeni refresh token varsa eskisinin yerini alır; yoksa mevcut token korunur. Google'ın her yenilemede rotation yapacağı veya tokenın 30 gün yaşayacağı varsayılmaz. Sonraki JWKS/Drive isteği başarısız olsa da döndürülmüş bundle saklanır.
5. Her cihazın Drive kaydı ve yerel oturumu ayrıdır. Yenileme mevcut, etkin cihaz üyeliğini gerektirir; silinen kayıt sessizce yeniden oluşturulmaz. Çıkış ve kendi cihazını kaldırma yerel çapayı ve bağlantıları da kapatır. Duraklatma yalnız çevrimdışı duruma geçer.
6. Hesap kimliği bağlantı izni değildir. Aynı hesap, canlı Drive üyeliği, QUIC el sıkışmasındaki gerçek anahtar ve ayrıca yerel cihaz onayı gerekir. İlk onayın izinleri korunur; iptal edilen kayıt doğrulama sırasında geri oluşturulamaz. Yerel onay iptali açık bağlantıyı da kapatır.

Google yenileme JWT'si nonce içermiyorsa gelen hesap kanıtı yalnız daha önce bağımsız onaylanmış QUIC anahtarında kabul edilebilir. İmza/audience/issuer/zaman kontrolleri ve canlı, tekil, etkin aynı hesap Drive kaydı zorunludur. Eksik nonce Google imzasının bir parçasıymış gibi doldurulmaz. Yeni/eşleşmemiş anahtarda normal OAuth kanıtı ve açık kullanıcı onayı gerekir. Protokol alanları, `SURUM=8` ve `afudesk/8` değişmez.

## Güvenli saklama

Windows mevcut CurrentUser DPAPI yolunu korur. Linux önce libsecret/Secret Service kullanır. Servis mevcut değilse AES-256-GCM ile şifreli dosya kullanılır; dosya ve ayrı rastgele anahtar 0600, özel dizin 0700 olmalıdır. Sahiplik, sembolik bağlantı ve hardlink denetimleri vardır; yazımlar fsync ve atomik yeniden adlandırmayla tamamlanır. Anahtar `renameat2(RENAME_NOREPLACE)` ile tam yazıldıktan sonra yayımlanır. İşlemler süre sınırlı süreçler arası dosya kilidiyle seri çalışır.

Fallback seçildikten sonra o kayıt için aynı backend kullanılır. Geri gelen keyring'in eski tokenı güncel fallback'in yerine geçmez. Çıkışta şifreli boş kayıt, eski keyring oturumunun geri gelmesini engeller. Linux GTK uygulaması tek örnek olarak açılır. Tokenlar kullanıcı arayüzüne, tanı kayıtlarına veya repoya yazılmaz.

Bu dosya şifrelemesi, anahtar dosyasını da okuyabilen aynı kullanıcı/root saldırısına karşı ayrı bir güven sınırı değildir; yerel saklama koruması işletim sistemi sahipliği ve dosya izinlerine dayanır. Kopyalanan bundle tek başına başka cihazda yetki vermez.

## Dış doğrulama sınırları

Gerçek Google hesabıyla tarayıcı girişi ve Console yayın durumu bu görevde doğrulanmadı. External/Testing durumunda Drive kapsamlı refresh tokenın yedi günlük sınırı olabilir; 30 günlük yerel güven bunu uzatmaz. [Google'ın refresh token süresi belgesi](https://developers.google.com/identity/protocols/oauth2#expiration) esas alınır. Console veya OAuth istemcileri değiştirilmedi.

Gerçek sonuç ve test komutları: [Linux test raporu](LINUX_TEST_RAPORU.md). Donanım/gerçek hesap denemesi mock testlerin yerine geçmiş sayılmaz.
