# AfuDesk Yol Haritası

Temel ilke: **kullanıcıdan teknik ayar isteme; ölç, en iyisini kendin seç.** Tek hedef metrik:
**input-to-pixel gecikmesi** (fare/tuş → uzak işlem → yakala → kodla → ağ → çöz → ekran).

## Yapılanlar
| Sürüm | Özellik |
|---|---|
| v1.0.0 | Tek kod + parola, doğrudan QUIC, onay/izin, 64×64 değişen döşeme JPEG, Windows |
| v1.1.0 | **Kare başına QUIC akışı** (baş-hat tıkanması yok), yetişmeyen kare iptal + döşemelerini yeniden gönderme, döşeme başına sıra (eski kare yeniyi ezmez), **otomatik kalite/FPS** (RTT + iptal sayısına göre), **gecikme göstergesi** (RTT · FPS), **Android izleyici** (dokunmatik kontrol, klavye çubuğu, yatay tam ekran) |

## Sıradakiler (etki / iş oranına göre)
1. **Kare zaman damgası ile gerçek gecikme ölçümü** — yakala→göster süresi (saat farkı RTT/2 ile düzeltilir). Optimize etmeden önce ölç.
2. **Windows Graphics Capture** (`windows-capture`) — GDI yerine GPU dokusundan yakalama.
3. **Donanımsal H.264** (NVENC / Quick Sync / AMF; B-frame yok, lookahead yok, kısa GOP). Sürekli değişen alanlarda (video/oyun) JPEG döşemelerin yerini alır.
4. **Pano ve dosya aktarımı** (ayrı düşük öncelikli akış).
5. **Çoklu monitör seçimi.**
6. **Android'den bağlantı verme** (MediaProjection + Kotlin köprüsü).

## Oyun Modu (tasarım aşaması)
- **Couch Co-op:** host oyunu çalıştırır, uzak oyuncunun gamepad'i host'ta **sanal gamepad** olarak görünür (Windows'ta ViGEm benzeri sürücü gerekir). Titreşim geri iletilir. Oyuncu slotları (P1..P4), kopan oyuncuya aynı slot.
- **Öncelik sırası:** 1) giriş gecikmesi 2) FPS 3) kararlılık 4) çözünürlük 5) kalite.
- **Trafik önceliği:** gamepad/klavye/fare > ses > kontrol > video > dosya.
- **Game Preflight / Host Advisor:** iki cihaz bağlandıktan sonra *gerçek yol üzerinde* kısa test (her yönde upload, RTT, jitter, kayıp) + encoder testi → Host Score (upload %40, RTT %20, jitter %10, kayıp %10, GPU encode %15, doğrudan yol %5). Öneri zorunlu değil ("Yine de diğer bilgisayarı kullan"). Sonuçlar otomatik başlangıç ayarı olur; oturum boyunca gerçek video akışından ölçmeye devam eder.
- **Sanal LAN (Hamachi benzeri):** Wintun TUN adaptörü + şifreli P2P + broadcast/multicast taşıma. Merkezi matchmaking kullanan oyunlarda işe yaramaz.

## Karar bekleyenler
- **Relay:** en hızlı yolu seçmek için relay adayları önerildi; relay bir sunucudur ve "sunucu yok" ilkesiyle çelişir. İsteğe bağlı katman olarak ayrıca karar verilmeli.
- **İnternet üzerinden NAT arkasında bağlantı:** bu oturumda uygulanmadı.
