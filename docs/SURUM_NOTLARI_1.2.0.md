**English** · [Türkçe](#türkçe)

## AfuDesk v1.2.0 — real latency, clipboard, and game mode

### New
- **Real latency measurement:** the host and viewer exchange clock samples so the displayed delay reflects the actual capture-to-display time, with the round-trip correction applied.
- **Lowest-RTT route selection:** when several connection candidates are available, AfuDesk selects the one with the lowest round-trip time.
- **Windows Graphics Capture:** Windows uses WGC first and keeps xcap as a fallback; the measured average was 19 ms per frame with WGC versus 113 ms with xcap.
- **Clipboard sharing:** text can be shared in both directions when clipboard permission is granted. Sharing is permission-based and disabled by default.
- **File transfer:** the viewer can send files to the host only after the host grants permission. Interrupted transfers resume from the verified partial file.
- **Game Mode:** a remote gamepad is exposed on the host as a virtual Xbox controller, with vibration feedback returned to the viewer. Game Mode requires the ViGEmBus driver: [ViGEmBus releases](https://github.com/nefarius/ViGEmBus/releases).

### Improved
- Latency statistics now use synchronized host/viewer clocks instead of relying only on connection timing.
- Windows capture can use the faster Graphics Capture path without changing the remote-viewing workflow.

---

## Türkçe

## AfuDesk v1.2.0 — gerçek gecikme, pano ve oyun modu

### Yeni
- **Gerçek gecikme ölçümü:** host ve izleyici saat örneklerini paylaşarak görüntünün yakalanmadan gösterime kadar geçen gerçek süreyi, gidiş-dönüş düzeltmesiyle birlikte ölçer.
- **En düşük RTT'li yol seçimi:** birden fazla bağlantı adayı olduğunda AfuDesk gidiş-dönüş süresi en düşük adayı seçer.
- **Windows Graphics Capture:** Windows önce WGC'yi kullanır ve xcap'i yedek yol olarak bırakır; ortalama ölçüm WGC için kare başına 19 ms, xcap için 113 ms oldu.
- **Pano paylaşımı:** pano izni verildiğinde metinler iki yönde paylaşılabilir. Paylaşım izinle açılır ve varsayılan olarak kapalıdır.
- **Dosya aktarımı:** izleyici, yalnızca host izin verdikten sonra host'a dosya gönderebilir. Kesilen aktarım doğrulanmış kısmi dosyadan devam eder.
- **Oyun Modu:** uzak oyun kolu host'ta sanal bir Xbox kolu olarak görünür; titreşim geri bildirimi izleyiciye döner. Oyun Modu için ViGEmBus sürücüsü gerekir: [ViGEmBus sürümleri](https://github.com/nefarius/ViGEmBus/releases).

### İyileştirmeler
- Gecikme istatistikleri artık yalnızca bağlantı zamanlamasına değil, eşitlenmiş host/izleyici saatlerine dayanır.
- Windows yakalama, uzak görüntüleme akışını değiştirmeden daha hızlı Graphics Capture yolunu kullanabilir.
