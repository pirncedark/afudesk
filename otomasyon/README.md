# AfuDesk otomasyon — adım 1–5

Node 20+, TypeScript ve resmi Playwright; AfuDesk çekirdeğinden bağımsız modül.

```sh
cd otomasyon
npm ci
npm run browser:install
npm test
```

`npm test`, TypeScript derlemesinin ardından Node'un tek süreçli test
çalıştırıcısını kullanır; tarayıcı testlerinde Playwright gerçek Chromium'u sürer.
Testler yalnız yerel HTTP sahte siteye erişir; Google istekleri engellenir.
Tarayıcı süreçleri başlatılamıyorsa test başarısız olur, atlanmaz.
Linux'ta gerekirse `npx playwright install --with-deps chromium` kullanılır.

## Kullanım

`siteler.json` varsayılan olarak boştur; site eklemek kullanıcı onayıdır.
`siteler.schema.json` ve çalışma zamanı doğrulaması yalnız tanımlı alanları kabul eder.

```json
{
  "siteler": [{
    "id": "ornek",
    "name": "Örnek Site",
    "url": "https://example.com/login",
    "permissions": ["oku", "giriş"],
    "loggedIn": { "selector": "[data-signed-in]" }
  }]
}
```

```ts
import { loadSites, openProfile, signIn } from './dist/src/index.js';

const context = await openProfile({ repoRoot: '/path/to/afudesk' });
try {
  const result = await signIn({
    context, sites: await loadSites('siteler.json'), siteId: 'ornek',
  });
  // UI yalnız result.message gösterir; stopped ise kullanıcı müdahalesi ister.
} finally { await context.close(); }
```

İlk Google girişini kullanıcı kendisi yapar: `manualLoginCommand({repoRoot})`,
aynı ayrı profil için Chrome komutunu otomasyon bayrakları olmadan döndürür;
bu komut yalnız kullanıcının “Google'a giriş yap” işlemiyle başlatılmalıdır.
Elle giriş penceresi kapanınca `openProfile` kullanılabilir; profil aynı anda
iki tarayıcıda açılmaz. Bu aşamada Flutter düğmesi/IPC yoktur (planın adım 6'sı).

Profil Windows'ta `%LOCALAPPDATA%\AfuDesk\otomasyon\profil`, Linux'ta
`~/.local/share/afudesk/otomasyon/profil` içindedir; repo içi ve repo içine
çözülen bağlantılı yollar reddedilir. Linux profil izni `700` yapılır.
Chrome yoksa Linux'ta Playwright Chromium yedeği denenir.

## Güvenlik ve gelişmiş site kuralları

Her iş için ayrı bir tarayıcı bağlamı kullanın. İzinler `oku`, `giriş`,
`form-doldur`, `hassas`; bu sürüm yalnız oturum kontrolü ve Google girişini yapar.
`oku` giriş düğmesine basamaz; form ve hassas işlemler daima onay için durur.
İzinli site dışına yönlendirme durdurulur; Google hesap alanı yalnız giriş için
kullanılabilir. İş sona erince çağıran bağlamı kapatmalıdır.

Oturum kuralı `loggedIn.selector` veya tam `loggedIn.url` olabilir; ikisi varsa
ikisi de gerekir. `googleButtonSelector`, `accountSelector`, `stopSelectors`
siteye göre özelleştirilebilir. Hesap yalnız onaylı `accountSelector` tek bir
görünür hesap gösteriyorsa seçilir; diğer durumlarda kullanıcıya bırakılır.

Consent, yeni hesap, CAPTCHA, 2FA, telefon ve ödeme ekranları (popup/iframe dahil)
öncelikle kontrol edilir; otomatik onay/çözüm/atlatma yapılmaz. Düzeni değişen
sitelerde seçiciler güncellenmelidir; bilinmeyen ekran başarılı sayılmaz.

Her sonuç `kayit/<site>/<UTC-tarih>.jsonl` içine yazılır: site kimliği, zaman,
giriş yolu, sabit durum/neden ve onay gereksinimi. Serbest metin, adres,
şifre, token, çerez, HTML ve ekran görüntüsü yazılmaz. Linux günlük izni `600`.
Profil, günlükler ve derleme çıktıları git tarafından yok sayılır.

Commit öncesi depo kökünde `gitleaks git --staged --config otomasyon/.gitleaks.toml`
çalıştırılır; varsayılan sır kuralları yanında profil/kayıt yolları da yasaktır.
