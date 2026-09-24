**English** · [Türkçe](#türkçe)

## AfuDesk v1.1.0 — Android viewer and smoother video

### New
- **Android app (viewer):** connect to your computer from your phone or tablet.
  - Tap = left click, long press = right click, drag = drag/select, two-finger swipe = scroll, two-finger tap = right click.
  - A cursor marker shows where you touched.
  - **Keyboard** button opens the phone keyboard; quick keys: Esc, Tab, Enter, Backspace, arrows, Win.
  - The session switches to landscape full screen automatically.
- **Latency indicator:** the session title shows round-trip time and frame rate (green < 80 ms, red > 200 ms).
- **Automatic quality:** image quality and frame rate adapt to the connection every second.

### Improved
- **Every frame travels on its own QUIC stream:** a lost or late frame no longer holds up the following ones. Frames that cannot keep up are cancelled and their screen areas are re-sent, so the picture never stays incomplete.
- An older frame arriving late can no longer overwrite newer content.

### Download
| Platform | File |
|---|---|
| Windows 10/11 (64-bit) | `AfuDesk-windows-x64.zip` — extract and run `afudesk.exe` |
| Android 7.0+ | `AfuDesk-android.apk` — allow "install unknown apps" for your browser/file manager |

SHA-256 checksums are in the `.sha256` files. On Android, sharing your phone's screen is not available yet; you can connect to a computer.

### Tests
43 Rust core tests (including real end-to-end QUIC), 46 UI tests (including 16 for touch control), 2 end-to-end tests on the real Windows engine, 11-screen visual check. The Android APK is built and signed in CI; it has not been tested on a physical device yet.

---

## Türkçe

## AfuDesk v1.1.0 — Android izleyici ve daha akıcı görüntü

### Yeni
- **Android uygulaması (izleyici):** telefondan ya da tabletten bilgisayarına bağlan.
  - Dokun = sol tık, uzun bas = sağ tık, sürükle = sürükleme/seçme, iki parmakla kaydır = tekerlek, iki parmakla dokun = sağ tık.
  - Dokunduğun yerde imleç işareti görünür.
  - **Klavye** düğmesi telefon klavyesini açar; hızlı tuşlar: Esc, Tab, Enter, Geri sil, oklar, Win.
  - Oturum otomatik olarak yatay tam ekrana geçer.
- **Gecikme göstergesi:** oturum başlığında gidiş-dönüş süresi ve kare hızı görünür (80 ms altı yeşil, 200 ms üstü kırmızı).
- **Otomatik kalite:** görüntü kalitesi ve kare hızı her saniye bağlantıya göre ayarlanır.

### İyileştirmeler
- **Her kare kendi QUIC akışında gider:** kaybolan ya da geciken bir kare artık sonrakileri bekletmez. Yetişemeyen kareler iptal edilir ve o ekran bölgeleri yeniden gönderilir; görüntü hiçbir zaman eksik kalmaz.
- Geç gelen eski bir kare artık yeni içeriğin üzerine yazamaz.

### İndir
| Platform | Dosya |
|---|---|
| Windows 10/11 (64 bit) | `AfuDesk-windows-x64.zip` — çıkar, `afudesk.exe`'yi çalıştır |
| Android 7.0+ | `AfuDesk-android.apk` — tarayıcına/dosya yöneticine "bilinmeyen uygulamaları yükle" izni ver |

SHA-256 değerleri `.sha256` dosyalarında. Android'de telefon ekranını paylaşmak henüz yok; bir bilgisayara bağlanabilirsin.

### Test
Rust çekirdeğinde 43 test (gerçek QUIC uçtan uca dahil), 46 arayüz testi (16'sı dokunmatik kontrol), gerçek Windows motorunda 2 uçtan uca test, 11 ekranlık görsel denetim. Android APK CI'da derlenip imzalandı; henüz gerçek bir cihazda denenmedi.
