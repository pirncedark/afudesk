# AfuDesk v1.2 Araştırma Raporu

**Araştırma tarihi:** 24 Eylül 2026  
**Kapsam:** Yalnızca dokümantasyon ve kaynak kodu incelemesi; kod yazılmadı, kurulum yapılmadı, git kullanılmadı.  
**Not:** Sürüm ve davranış bilgileri crates.io, docs.rs, ilgili proje depoları ve Microsoft/IETF belgelerinden karşılaştırılarak yazıldı. Kaynakta açıkça doğrulanamayan noktalar açıkça belirtilmiştir.

## 1. `windows-capture` ile birincil monitörü sürekli yakalama

### Kimlik ve güncel sürüm

- **Crate:** [`windows-capture`](https://crates.io/crates/windows-capture)
- **Kaynak:** [NiiightmareXD/windows-capture](https://github.com/NiiightmareXD/windows-capture)
- **Güncel sürüm:** `2.0.1` (crates.io’da 8 Ağustos 2026’da yayımlanmış; araştırma tarihinde crates.io’nun güncel sürümü)
- **Lisans:** MIT
- **Belgeler:** [docs.rs/windows-capture/2.0.1](https://docs.rs/windows-capture/2.0.1/windows_capture/)

### Kısa çalışır örnek

Aşağıdaki örnek birincil monitörü seçer, `Rgba8` yakalama başlatır ve ilk kareyi padding’siz `Vec<u8>` olarak kopyalar. `start` mevcut iş parçacığını yakalama döngüsüne adayır; GUI uygulamasında `start_free_threaded` ve `CaptureControl` daha uygun olabilir.

```rust
use std::error::Error;
use windows_capture::capture::{Context, GraphicsCaptureApiHandler};
use windows_capture::frame::Frame;
use windows_capture::graphics_capture_api::InternalCaptureControl;
use windows_capture::monitor::Monitor;
use windows_capture::settings::{
    ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
    MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
};

struct Capture;

impl GraphicsCaptureApiHandler for Capture {
    type Flags = (u32, u32);
    type Error = Box<dyn Error + Send + Sync>;

    fn new(_ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self)
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let mut mapped = frame.buffer()?;
        let mut rgba = Vec::with_capacity((frame.width() * frame.height() * 4) as usize);
        let pixels = mapped.as_nopadding_buffer(&mut rgba).to_vec();
        assert_eq!(pixels.len(), (frame.width() * frame.height() * 4) as usize);
        let _rgba_vec: Vec<u8> = pixels;
        control.stop();
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let monitor = Monitor::primary()?;
    let size = (monitor.width()?, monitor.height()?);
    let settings = Settings::new(
        monitor,
        CursorCaptureSettings::WithCursor,
        DrawBorderSettings::WithoutBorder,
        SecondaryWindowSettings::Exclude,
        MinimumUpdateIntervalSettings::Default,
        DirtyRegionSettings::Default,
        ColorFormat::Rgba8,
        size,
    );
    Capture::start(settings)?;
    Ok(())
}
```

`ColorFormat::Rgba8` 8-bit RGBA’dır ve varsayılan biçimdir. `Frame::buffer()` GPU dokusunu CPU’ya map eder; ham arabellekte satır sonu hizalama (`row_pitch`) bulunabilir. `as_raw_buffer()` padding içerebilir. `as_nopadding_buffer(&mut vec)` satır padding’ini ayıklayıp kopyalar. Kopyalama maliyeti yüksekse `as_nopadding_buffer` yerine ham arabellek ve doğru `row_pitch` kullanılabilir.

`Monitor::primary()` `MonitorFromPoint(0, 0)` ile birincil monitörü seçer ve `Monitor`, `GraphicsCaptureItem` üretmek için `TryInto` dönüşümü destekler. Monitör yakalama için ayrıca sistem picker’ı gerekmez; picker tabanlı akış `GraphicsCapturePicker::pick_item()` ile ayrıca kullanılabilir.

### İlk kare gecikmesi

`windows-capture` ve Microsoft dokümanları sabit bir “ilk kare şu kadar milisaniye sonra gelir” değeri vermiyor. `GraphicsCaptureApiHandler::on_frame_arrived` olay tabanlıdır; kare, capture session ve frame pool hazırlandıktan sonra Windows compositor/D3D akışından gelir. Bu nedenle:

- `StartCapture()` çağrısından sonra `on_frame_arrived` anında gelmeyebilir.
- İlk kare gecikmesi donanım, GPU, compositor, pencere güncellemesi, sistem yükü ve API sürümüne bağlıdır.
- “İlk kare bekleniyor” durumunda sabit süre beklemek yerine başlangıç zamanını ölçüp timeout uygulamak gerekir.
- `start_free_threaded` yalnızca yakalamanın ayrı bir görevde çalışmasını sağlar; ilk kare gecikmesini ortadan kaldırmaz.
- `MinimumUpdateIntervalSettings::Custom`, sabit FPS garantisi değildir; yalnızca uygun güncellemelerin minimum aralığını sınırlar. Sabit ritim isteniyorsa uygulama kendi saatini kullanmalı, fazla kareyi düşürmeli veya son kareyi tekrarlamalıdır.

2.0.1 kaynak README’sinde DXGI Desktop Duplication örneğinde ilk birkaç çağrının boş kare döndürebileceği ayrıca belirtilir. Bu davranış **DXGI Desktop Duplication** yoluna ilişkindir; yukarıdaki Graphics Capture handler yolunda da sabit ilk-kare garantisi yoktur.

### Sarı çerçeve kapatma

- `DrawBorderSettings::Default`: Windows varsayılanını kullanır.
- `DrawBorderSettings::WithBorder`: çerçeve ister.
- `DrawBorderSettings::WithoutBorder`: çerçeve istemez.
- Microsoft, Graphics Capture’ın aktif yakalama göstergesi olarak yakalanan ekran veya pencere çevresine sarı bildirim çerçevesi çizdiğini belirtir.
- `IsBorderRequired=false` özelliği Windows 10 2104 / API contract v12 seviyesinde belgelenmiştir. Çerçevesiz yakalamanın gerçekten devre dışı bırakılması için paketlenmiş uygulamada `graphicsCaptureWithoutBorder` manifest capability’si ve kullanıcı onayı gerekir.
- Kullanıcı onayı reddedilirse veya başka bir uygulama aynı nesne için çerçeve istiyorsa çerçeve görünmeye devam edebilir. `WithoutBorder` seçmek tek başına garantili borderless görüntü demek değildir.
- `windows-capture` bu özellik desteklenmiyorsa ayar uygulanmadan önce destek kontrolü yapabilir; eski Windows sürümünde sürüm kontrolü yapılmalıdır.

### İmleci dahil etme veya etmeme

- `CursorCaptureSettings::WithCursor`: imlecin yakalanmasını ister.
- `CursorCaptureSettings::WithoutCursor`: imlecin yakalanmamasını ister.
- `CursorCaptureSettings::Default`: sistem varsayılanını bırakır.
- Microsoft `IsCursorCaptureEnabled` özelliğini Windows 10 2004 / build 19041 seviyesinde belgeliyor. Bu nedenle eski sistemlerde `WithoutCursor` desteği ayrıca doğrulanmalıdır.
- İmleci kapatmak, fare işaretini veya oyun içi cursor rendering’i kapatmaz; yalnızca capture içeriğine imleç piksellerinin eklenip eklenmediğini etkiler.

### Windows 10 sürüm gereksinimi

`windows-capture 2.0.1` için crate dokümanı tek bir paket-spesifik minimum Windows sürümü yayımlamıyor. Doğrulanabilen katmanlar şöyledir:

- `Windows.Graphics.Capture.GraphicsCaptureItem` temel API ailesi Windows 10 1803, build 17134 ile belgelenmiştir.
- `GraphicsCaptureSession` temel desteği de Windows 10 1803 seviyesindedir.
- `IsCursorCaptureEnabled` Windows 10 2004, build 19041 gerektirir.
- `IsBorderRequired` Windows 10 2104, build 20348 gerektirir.
- Win32 `CreateForWindow` yolu kullanan bazı Microsoft örnekleri Windows 10 1903 gerektirir; bu, temel `Windows.Graphics.Capture` API’sinin 1803 gereksiniminden farklıdır.
- Sonuç: uygulamanın destekleyeceği Windows build’ı özellik bazında belirlenmelidir. `windows-capture 2.0.1` için tek bir garantili minimum build bu araştırmada doğrulanmadı.

### Bilinen tuzaklar

- `as_raw_buffer()` satır sonu padding’i içerebilir; `rgba.len() == width * height * 4` varsaymak ham arabellekte yanlış olabilir.
- `Rgba8` isteniyorsa `Bgra8` karışık gelirse renk kanalları değişir. `ColorFormat` her frame’de sabit tutulmalıdır.
- Frame’i saklamak frame pool’e iade edilmeyi geciktirir; GPU kaynaklarını sızdırmamak için `Frame` yaşam süresi kısa tutulmalıdır.
- GPU’dan CPU’ya kopyalama pahalıdır; özellikle sürekli 4K yakalamada donanım ve CPU bütçesi ölçülmelidir.
- `MinimumUpdateIntervalSettings` sabit FPS sözü vermez.
- Monitör çözünürlüğü/refresh rate değişirse frame pool yeniden oluşturulmalı ve boyut yeniden alınmalıdır.
- `WithoutCursor` ve `WithoutBorder` özellikleri işletim sistemi sürümüne ve kullanıcı onayına bağlıdır.
- Paketlenmemiş Rust masaüstü uygulamasında borderless consent/manifest gereksinimi için crate’nin sunduğu ayar yeterli olmayabilir; hedef dağıtım biçimi doğrulanmalıdır.
- HDR içerikte `Rgba8` ton eşlemesini bozabilir; `Rgba16F` ve HDR işleme ayrıca değerlendirilmelidir.

### DENETIM ICIN KONTROL:

- `Monitor::primary()` kullanılıyor ve monitörün gerçekten birincil olduğu test ediliyor mu?
- Her frame’de `ColorFormat`, `width`, `height`, `row_pitch` ve padding durumu kontrol ediliyor mu?
- `as_raw_buffer()` yerine `as_nopadding_buffer()` ile istenen RGBA `Vec<u8>` elde ediliyor mu?
- `WithCursor`/`WithoutCursor` ve `WithBorder`/`WithoutBorder` seçimleri hedef Windows build’ında gerçekten destekleniyor mu?
- Session başlangıcından ilk `on_frame_arrived` çağrısına kadar timeout, yeniden başlatma ve frame-pool yeniden oluşturma senaryoları var mı?

## 2. `vigem-client` ile sanal Xbox 360 kolu

### Kimlik ve güncel sürüm

- **Crate:** [`vigem-client`](https://crates.io/crates/vigem-client)
- **Kaynak:** [CasualX/vigem-client](https://github.com/CasualX/vigem-client)
- **Güncel sürüm:** `0.1.4` (son yayın 1 Ağustos 2022; araştırma tarihinde crates.io’da daha yeni sürüm yok)
- **Lisans:** MIT
- **Belgeler:** [docs.rs/vigem-client/0.1.4](https://docs.rs/vigem-client/0.1.4/vigem_client/)
- **Önemli özellik:** `unstable_xtarget_notification` varsayılan olarak kapalıdır.

### Kısa çalışır örnek

Sadece sanal kol oluşturma ve `XGamepad` gönderme:

```rust
use vigem_client::{Client, TargetId, XButtons, XGamepad, Xbox360Wired};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::connect()?;
    let mut target = Xbox360Wired::new(client, TargetId::XBOX360_WIRED);
    target.plugin()?;
    target.wait_ready()?;

    let gamepad = XGamepad {
        buttons: XButtons!(A | X),
        left_trigger: 0,
        right_trigger: 255,
        thumb_lx: 12_345,
        thumb_ly: -6_789,
        thumb_rx: 0,
        thumb_ry: 0,
    };
    target.update(&gamepad)?;
    Ok(())
}
```

Titreşim/LED bildirimi almak için özellik açılmalıdır:

```toml
[dependencies]
vigem-client = { version = "0.1.4", features = ["unstable_xtarget_notification"] }
```

```rust
use vigem_client::{Client, TargetId, XGamepad, Xbox360Wired};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::connect()?;
    let mut target = Xbox360Wired::new(client, TargetId::XBOX360_WIRED);
    target.plugin()?;
    target.wait_ready()?;

    let state = XGamepad::default();
    target.update(&state)?;

    let thread = target.request_notification()?.spawn_thread(|_, notification| {
        println!(
            "large_motor={}, small_motor={}, led={}",
            notification.large_motor,
            notification.small_motor,
            notification.led_number
        );
    });

    drop(target);
    thread.join().unwrap();
    Ok(())
}
```

### `XGamepad` alanları

`XGamepad`, `XINPUT_GAMEPAD` uyumlu bir rapor yapısıdır:

- `buttons: XButtons`: bit bayrakları; `XButtons!(A | X)` gibi kullanılır.
- `left_trigger: u8`: sol tetik, dinlenen aralık `0..=255`.
- `right_trigger: u8`: sağ tetik, dinlenen aralık `0..=255`.
- `thumb_lx: i16`: sol çubuk X.
- `thumb_ly: i16`: sol çubuk Y.
- `thumb_rx: i16`: sağ çubuk X.
- `thumb_ry: i16`: sağ çubuk Y.

Buton bitleri XInput düzenindedir: D-pad `UP/DOWN/LEFT/RIGHT`, `START/BACK`, thumb button’ları, `LB/RB/GUIDE`, yüz butonları `A/B/X/Y`. Çubukların negatif değerleri aşağı/sol gibi cihaz yönlerine bağlıdır; uygulama aynı işaret konvansiyonunu iki tarafta da sabitlemelidir.

### Titreşim bildirimi

`unstable_xtarget_notification` açıldığında crate şu yapıyı sunar:

- `Xbox360Wired::request_notification() -> XRequestNotification`
- `XRequestNotification::poll(wait) -> Result<Option<XNotification>, Error>`
- `XRequestNotification::spawn_thread(callback)` yardımcısı
- `XNotification { large_motor: u8, small_motor: u8, led_number: u8 }`

Bu, `XGamepad` içinde motor alanı olan bir gönderim API’si değildir. Sanal kolun gönderdiği input raporunda motor alanı bulunmaz. Bildirim, ViGEm hedefine XInput notification isteği verildiğinde dışarıdan gelen motor/LED durumunu okur. Örnek kaynakta motor değerleri XInput tarafında `u16` olarak üretilir ancak ViGEm notification yapısında yalnızca yüksek byte kullanılır ve crate API’si `u8` döndürür.

Örnekte ayrıca `rusty_xinput` ile `xinput.set_state(...)` çağrılır. Bu, `vigem-client` API’sinin bir parçası değildir; yalnızca test/örnek akışını gösterir. Test sırasında XInput motor komutu üreten ayrı bir yol gerekir.

### ViGEmBus sürücüsü yoksa hata ve tespit

- `vigem-client`, ViGEm C client kütüphanesini kullanmayan saf Rust bir istemcidir.
- Sürücü kurulu değilse `Client::connect()` hata döndürür ve tipik hata `Error::BusNotFound` olur.
- Sürücü bulunur fakat erişim başarısız olursa `Error::BusAccessFailed(code)` döner.
- Sürücü sürümü istemciyle uyumsuzsa `Error::BusVersionMismatch` döner.
- Başka hatalar: `NoFreeSlot`, `AlreadyConnected`, `NotPluggedIn`, `TargetNotReady`, `UserIndexOutOfRange`, `OperationAborted`, `WinError(code)`.
- Kütüphanede ayrı bir `is_driver_installed()` veya “sürücü kurulu mu?” yardımcı fonksiyonu yoktur. En güvenilir tespit `Client::connect()` çağrılıp hata eşleştirilmesidir:

```rust
match vigem_client::Client::connect() {
    Ok(_) => println!("ViGEmBus erişilebilir"),
    Err(vigem_client::Error::BusNotFound) => println!("ViGEmBus kurulu değil"),
    Err(vigem_client::Error::BusAccessFailed(code)) => println!("erişim hatası: {code}"),
    Err(e) => println!("farklı hata: {e:?}"),
}
```

`plugin()` sonrası `wait_ready()` çağrılmadan `update()` yapılırsa `TargetNotReady` oluşabilir. Target drop edildiğinde otomatik unplug olur; süreç ani sonlanırsa sanal cihaz Windows’ta takılı kalabilir.

### Bilinen tuzaklar

- ViGEmBus sürücüsü crate ile birlikte gelmez; kullanıcı ayrıca kurmalıdır.
- `Client::connect()` başarısızlığını yalnızca boolean olarak görmek hangi hatanın sürücü eksikliği, erişim veya sürüm uyuşmazlığı olduğunu ayırt etmez.
- `plugin()` ile `wait_ready()` arasında update gönderilmemelidir.
- Hedef başlangıçta plug edilmiş değildir; yeni target üzerinde `NotPluggedIn` normaldir.
- `unstable_xtarget_notification` kararsız/feature-gated API’dir; varsayılan bağımlılıkla erişilemez.
- Bir hedef için birden fazla notification request oluşturulmamalıdır.
- Notification kaybolabilir veya birden fazla listener’a ulaşabilir.
- Target unplug edilirse bekleyen notification `OperationAborted` ile sonlanabilir.
- Motor bildirimi `XNotification` alanlarında `u8` olarak gelir; 16-bit XInput değerinin tamamı değildir.
- Sanal kolu oluşturmak gerçek bir fiziksel cihaz veya fiziksel motor eklemez; yalnızca Windows XInput tarafına sanal hedef sunar.

### DENETIM ICIN KONTROL:

- `Client::connect()` hatası `BusNotFound`, `BusAccessFailed` ve `BusVersionMismatch` olarak ayrı ayrı raporlanıyor mu?
- Her target için `plugin()` sonrasında `wait_ready()` çağrılıyor ve hazır olmadan `update()` denenmiyor mu?
- `XGamepad` alanlarının tümü (`buttons`, iki trigger, dört `thumb_*`) doğru aralıklarla dolduruluyor mu?
- Bildirim kullanılıyorsa `unstable_xtarget_notification` özelliği bilinçli olarak açılmış ve target başına tek request kullanılıyor mu?
- Target drop/unplug veya process sonlanması durumunda takılı sanal cihaz temizliği planlanmış mı?

## 3. `gilrs` ile Windows’ta XInput kolu okuma

### Kimlik ve güncel sürüm

- **Crate:** [`gilrs`](https://crates.io/crates/gilrs)
- **Kaynak:** [gilrs-project/gilrs](https://gitlab.com/gilrs-project/gilrs)
- **Güncel sürüm:** `0.11.2` (crates.io’da 30 Mayıs 2026’da yayımlanmış)
- **Lisans:** Apache-2.0 veya MIT (`Apache-2.0/MIT`)
- **Belgeler:** [docs.rs/gilrs/0.11.2](https://docs.rs/gilrs/0.11.2/gilrs/)
- **MSRV:** 0.11.2 için crates.io kaynak manifesti `1.84.0` gösteriyor.

### XInput arka uç seçimi

Windows’ta `gilrs` varsayılan olarak Windows Gaming Input (`wgi`) arka uçunu kullanır. Terminal uygulaması veya odaklı pencereyle ilişkilendirilmek istemeyen bir servis kullanılıyorsa XInput arka ucu açıkça seçilmelidir:

```toml
[dependencies]
gilrs = { version = "0.11.2", default-features = false, features = ["xinput"] }
```

`wgi`, olayların güvenilir biçimde alınması için process ile ilişkili ve focus alan bir pencere gerektirebilir. XInput arka ucu bu pencere bağımlılığını azaltır. Her iki arka uç da Windows’ta hotplug ve force feedback desteğine sahiptir; cihazın gerçek rumble desteği yine cihaz kapasitesidir.

### Kısa çalışır örnek

```rust
use gilrs::{Axis, EventType, Gilrs};
use std::time::Duration;

fn main() {
    let mut gilrs = Gilrs::new().unwrap();

    loop {
        while let Some(event) = gilrs.next_event() {
            match event.event {
                EventType::ButtonPressed(button, _) => {
                    println!("button pressed: {button:?}");
                }
                EventType::AxisChanged(axis, value, _) => {
                    println!("axis {axis:?} = {value}");
                }
                EventType::Connected(id) => println!("connected: {id}"),
                EventType::Disconnected(id) => println!("disconnected: {id}"),
                _ => {}
            }
        }

        if let Some((_id, gamepad)) = gilrs.gamepads().next() {
            let value = gamepad.value(Axis::LeftStickX).clamp(-1.0, 1.0);
            let x = if value < 0.0 {
                (value * 32_768.0) as i16
            } else {
                (value * 32_767.0) as i16
            };
            let _xinput_i16 = x;
        }

        let _ = gilrs.next_event_blocking(Some(Duration::from_millis(16)));
    }
}
```

`next_event()` non-blocking’dir ve olay yoksa `None` döner. `next_event_blocking(Some(timeout))` olay bekler; bu yöntem web/Wasm’da desteklenmez ve orada panic verebilir. Normal oyun/async döngüsünde `next_event()` ve uygun bekleme stratejisi kullanılmalıdır.

### Eksen işaret ve ölçek kuralı

Gilrs normalize edilmiş değer üretir:

- Değer aralığı `-1.0..=1.0`.
- `Axis::LeftStickX`, `LeftStickY`, `RightStickX`, `RightStickY` analog çubuk eksenleridir.
- `LeftZ` ve `RightZ` ilgili çubuğun ikinci ekseni olarak kullanılabilir; `Axis::is_stick()` yalnızca dört `StickX/StickY` eksenini analog çubuk olarak işaretler.
- `DPadX` ve `DPadY` ayrı D-pad eksenleridir.
- `Axis::second_axis()` eşleşen diğer ekseni döndürür.

XInput `i16` ile eşleştirme için önerilen uygulama yaklaşımı:

- `v >= 0.0`: `round(v * 32767)`, sonuç `0..=32767`.
- `v < 0.0`: `round(v * 32768)`, sonuç `-32768..=0`.
- Girdi önce `-1.0..=1.0` aralığına clamp edilmelidir.
- Negatif float değeri Rust `as i16` dönüşümünde saturate olsa da uygulama sınırlarını açıkça ele almalıdır.
- `round` sonucu i16 aralığını aşarsa clamp uygulanmalıdır.
- Bazı cihazlar fiziksel eksen yönünü farklı raporlar; yalnızca `-1/+1` işaretini körlemesine çevirmek yerine cihaz/mapping davranışı test edilmelidir.

Bu dönüşüm ters yönde yapılacaksa tam tersi kullanılmalıdır: `i16` değeri `0` merkez kabul edilerek normalize edilir; negatif taraf `-32768`, pozitif taraf `32767` ile eşlenmelidir. Trigger değerleri de aynı şekilde `0..255` aralığına çevrilebilir.

### Force feedback

Windows’ta force feedback desteği vardır ve cihaz bazında `Gamepad::is_ff_supported()` ile kontrol edilir. `gilrs::ff` modülü `EffectBuilder`, `BaseEffect`, `BaseEffectType`, `Replay`, `Ticks` ve `Effect` sunar. Efekt `gamepads(&[id])` ile cihazlara bağlanır ve `effect.play()` ile oynatılabilir. `set_listener_position` da mevcuttur.

Kısa kontrol örneği:

```rust
use gilrs::Gilrs;

fn main() {
    let gilrs = Gilrs::new().unwrap();
    for (id, gamepad) in gilrs.gamepads() {
        println!("{id}: rumble_supported={}", gamepad.is_ff_supported());
    }
}
```

### Bilinen tuzaklar

- Varsayılan `wgi` arka ucu odaklı pencere olmadan tüm cihazlarda beklenen olayları üretmeyebilir; terminal/headless kullanım için `xinput` seçilmeli.
- `gilrs` tüm cihazları XInput 0-3 mantığıyla değil, SDL uyumlu normalize edilmiş bir layout ile sunar.
- `Axis` değerleri `-1.0..=1.0`; doğrudan `i16` değildir.
- Trigger ve analog çubuk aynı ölçekte değildir; trigger için `0..255`/`0..1` dönüşümü gerekir.
- Cihazın `is_ff_supported()` değeri false ise efekt oluşturma denemek başarısız olabilir; destek kontrolü cihaz seviyesinde yapılmalıdır.
- `next_event()` yalnızca mevcut olayları tüketir; event loop dışında çalışan kodda `next_event_blocking` kullanılmalıdır.
- `next_event_blocking` Wasm’da desteklenmiyor ve panic verebilir.
- Hotplug sırasında `gamepads()` anlık görüntüsü değişebilir; bağlantı/ayrılma olaylarını aynı döngüde işlemek gerekir.
- FF efektlerinin çalışma süresi, gücü ve uygulama kapatıldığında etkinliğin sona ermesi cihaz ve sürücüye bağlıdır.
- XInput arka ucu seçimi crate özelliği olarak verilmelidir; yalnızca runtime’da “XInput” denetlemek arka uc değiştirmez.

### DENETIM ICIN KONTROL:

- Cargo feature’larında `wgi` kapatılıp `xinput` açıkça etkinleştirildi mi?
- Event loop hem `ButtonPressed/Released`, `AxisChanged`, `Connected` hem `Disconnected` olaylarını tüketiyor mu?
- Her normalize eksen clamp ediliyor ve negatif/pozitif uçlar i16 aralığına doğru eşleniyor mu?
- `is_ff_supported()` kontrol ediliyor ve desteklenmeyen cihazda efekt zorlanmıyor mu?
- Oyun/headless kullanımında `next_event_blocking` yerine uygun non-blocking bekleme stratejisi seçildi mi?

## 4. `quinn` 0.11 datagram

### Kimlik ve güncel sürüm

- **Crate:** [`quinn`](https://crates.io/crates/quinn)
- **Kaynak:** [quinn-rs/quinn](https://github.com/quinn-rs/quinn)
- **Güncel sürüm:** `0.11.12` (crates.io’da 14 Eylül 2026’da yayımlanmış)
- **0.11 serisinin en güncel sürümü:** `0.11.12`
- **Lisans:** MIT veya Apache-2.0
- **Belgeler:** [docs.rs/quinn/0.11.12](https://docs.rs/quinn/0.11.12/quinn/)

Bu raporda API açıklamaları araştırma tarihindeki `0.11.12` sürümüne göredir. Daha eski bir `0.11.x` kilitle kullanılıyorsa özellikle `quinn-proto` bağımlılık sürümü ve release notes kontrol edilmelidir.

### `TransportConfig` ayarları

`TransportConfig::default()` üzerinde şu iki ayar kullanılabilir:

```rust
use quinn::TransportConfig;

fn datagram_transport_config() -> TransportConfig {
    let mut config = TransportConfig::default();
    config.datagram_receive_buffer_size(Some(64 * 1024));
    config.datagram_send_buffer_size(64 * 1024);
    config
}
```

- `datagram_receive_buffer_size(Option<usize>)`: gelen uygulama datagramlarının toplam byte tampon sınırıdır. `None` gelen datagram desteğini kapatır.
- Peer tek bir datagram için bu değerden büyük gönderim yapamaz. Gelen ama uygulama tarafından tüketilmemiş datagramların toplamı sınırı aşarsa eski datagramlar düşürülür.
- `datagram_send_buffer_size(usize)`: giden uygulama datagramları için bellek sınırıdır. Link veya donanım uygulamanın üretim hızından yavaşsa yeni datagram için yer açarken eski gönderilmemiş datagramlar düşürülebilir.
- Buffer değerleri payload için üst sınırdır; QUIC/Datagram ve transport overhead nedeniyle kullanılabilir payload kapasitesi daha küçük olabilir.
- `TransportConfig` içinde sabit bir `max_datagram_size` alanı yoktur. Gönderilebilecek maksimum boyut bağlantı üzerinde `Connection::max_datagram_size()` ile dinamik ölçülür.

### Gönderme ve alma

```rust
use bytes::Bytes;
use quinn::Connection;

fn send_ping(connection: &Connection) -> Result<(), quinn::SendDatagramError> {
    let payload = Bytes::from_static(b"ping");
    if let Some(limit) = connection.max_datagram_size() {
        assert!(payload.len() <= limit);
    }
    connection.send_datagram(payload)
}

async fn receive_one(connection: &Connection) -> Result<Vec<u8>, quinn::ConnectionError> {
    let bytes = connection.read_datagram().await?;
    Ok(bytes.to_vec())
}
```

`send_datagram(Bytes)`:

- Uygulama datagramı gönderir.
- Güvenilir değildir; kaybolabilir veya sırasız teslim edilebilir.
- Tek QUIC paketine sığmalı ve peer’ın izin verdiği boyuttan küçük olmalıdır.
- Gönderim tamponu doluysa yeni datagram için eski gönderilmemiş datagramlar atılabilir.

`send_datagram_wait(Bytes)`:

- Tıkanıklık veya gönderim tamponu dolduğunda beklemecesine benzer.
- Yeni datagramları bekleyen eski datagramlara öncelik verir.
- Sonuç yine `SendDatagramError` döner; bu çağrı datagramı güvenilir hale getirmez.

`read_datagram()`:

- Gelen bir datagramı `Result<Bytes, ConnectionError>` olarak döndürür.
- Datagram sınırı ve sırası garanti edilmez.
- Uygulama sırası veya bütünlük gerekiyorsa stream kullanılmalıdır.

### `max_datagram_size` ve hatalar

`Connection::max_datagram_size() -> Option<usize>`:

- Datagram desteği yerel olarak kapalıysa veya peer desteklemiyorsa `None` döner.
- Path MTU ve peer’ın sabit sınırına göre bağlantı ömrü boyunca değişebilir.
- Gönderilebilir maksimum boyut, alınabilir maksimum boyutla aynı olmak zorunda değildir.

`SendDatagramError` varyantları:

- `TooLarge`: payload path MTU eksi overhead veya peer limitini aşıyor.
- `Disabled`: datagram desteği yerel olarak kapalı.
- `UnsupportedByPeer`: peer datagram frame’lerini desteklemiyor.
- `ConnectionLost(ConnectionError)`: bağlantı kaybedildi.

`send_datagram` sırasında eski datagramların düşürülmesi `TooLarge` anlamına gelmez. `TooLarge` payload sınırı/uyumluluk hatasıdır; tampon dolması ise sessiz veri kaybına yol açabilen ayrı davranıştır. `datagram_send_buffer_space()` sıfırdan büyükse, o boyuttan küçük bir datagramın gönderimi eski datagramları düşürmeden kabul edileceğinin garantisini verir.

### Güvenilir akışla birlikte kullanım

Aynı `Connection` üzerinde:

- `open_uni()`/`accept_uni()` ve `open_bi()`/`accept_bi()` ile güvenilir, sıralı stream’ler açılabilir.
- `send_datagram()`/`read_datagram()` ile küçük, gecikmeye duyarlı, kayıp kabul edilebilir mesajlar taşınabilir.
- İkisi aynı QUIC bağlantısında ve aynı congestion control altında çalışır.
- Datagram güvenilir akışın sırasını, teslim garantisini veya stream backpressure davranışını değiştirmez.
- Uzun ve kesinlikle ulaşması gereken state/mesajlar stream’e, hızlı güncelleme/pozisyon/ping gibi düşük gecikme mesajları datagram’a konmalıdır.

Örnek semantik:

```rust
async fn mixed_io(connection: &quinn::Connection) -> Result<(), Box<dyn std::error::Error>> {
    connection.send_datagram(bytes::Bytes::from_static(b"pose"))?;

    let (mut send, _recv) = connection.open_bi().await?;
    send.write_all(b"durable-state").await?;
    send.finish()?;
    Ok(())
}
```

### Bilinen tuzaklar

- Datagram tek pakete sığmalıdır; büyük kareyi olduğu gibi datagram olarak göndermek `TooLarge` veya parçalanma problemi doğurur.
- `max_datagram_size()` sabit bir config değeri değildir; MTU değişiminde değişebilir.
- `datagram_receive_buffer_size(None)` gelen datagramları kapatır; peer’ın tek datagram gönderim limiti bundan etkilenebilir. Bu, `UnsupportedByPeer` ile aynı hata değildir: destek协商 edilmiş olsa bile yerel receive buffer’ı kapalıdır.
- Çok yüksek üretim hızında gönderim tamponu eski datagramları sessizce düşürebilir.
- `send_datagram()` başarılı olmak teslim edildiği anlamına gelmez.
- `send_datagram_wait()` yalnızca tampon alanı bekler; ağ kaybı veya peer teslim garantisi sağlamaz.
- Datagram ve stream aynı congestion control ile yarıştığı için yüksek hacimli stream trafiği datagram gecikmesini artırabilir.
- QUIC datagram desteği handshake sırasında negotiate edilen özelliktir; desteklenmeyen peer için `UnsupportedByPeer` beklenmelidir.
- `TooLarge`, `Disabled`, `UnsupportedByPeer` ve bağlantı kapanışı aynı hata olarak ele alınıp sessizce yeniden gönderim döngüsüne sokulmamalıdır.
- `quinn` 0.11.12’nin `quinn-proto` bağımlılığı 0.11.18’dir; yalnızca `quinn` sürümüne bakıp transport davranışını sürümlendirmek eksik kalabilir.

### DENETIM ICIN KONTROL:

- `TransportConfig` içinde receive/send datagram buffer’ları amaca göre ayarlanmış ve `None` ile gelen desteğin kapatıldığı durumlar görülüyor mu?
- Gönderimden önce `Connection::max_datagram_size()` kontrol ediliyor ve payload tek pakete sığacak şekilde sınırlandırılıyor mu?
- `TooLarge`, `Disabled`, `UnsupportedByPeer` ve `ConnectionLost` ayrı ayrı ele alınıyor mu?
- Kritik mesajlar stream’e, gecikmeye duyarlı küçük mesajlar datagram’a yerleştiriliyor mu?
- Aynı bağlantıda stream ve datagram trafiğinin congestion/backpressure birlikte çalıştığı test ediliyor mu?

## 5. Uzak masaüstünde saat farkı ölçümü

### Kapsam, standart ve “crate” durumu

Bu madde bir Rust kütüphanesi değil, bir zaman protokolü/ölçüm problemidir. Bu nedenle **crate linki ve crate lisansı yoktur; “doğrulanmadı” olarak işaretlenir.** İlgili standart:

- **Protokol:** NTPv4
- **Ana standart:** [RFC 5905 — Network Time Protocol Version 4](https://www.rfc-editor.org/rfc/rfc5905)
- **Güncel standart durumu:** RFC 5905 Proposed Standard; aynı RFC’nin güncel errata/ek referansları dikkate alınmalıdır.
- **Güvenlik:** [RFC 8915 — Network Time Security](https://www.rfc-editor.org/rfc/rfc8915)
- **Varsayılan NTP portu:** UDP/123

`NTPv4`, uzak masaüstü uygulamasının saat farkını ölçmek için doğrudan bir uygulama protokolü değildir. İki bilgisayarın saatlerini karşılaştırmak için ortak veya güvenilir bir referans saate ihtiyaç vardır.

### Tek gidiş-dönüş formülü

İstemci bir zaman isteğini `T1` anında gönderir, sunucu `T2`’de alır ve `T3`’te yanıtı gönderir, istemci yanıtı `T4` anında alır:

```text
RTT = (T4 - T1) - (T3 - T2)
offset = ((T2 - T1) + (T3 - T4)) / 2
```

Rust’ta süreler `Duration` olarak tutuluyorsa örnek biçim şöyle olabilir:

```rust
use std::time::Duration;

fn offset_seconds(t1: Duration, t2: Duration, t3: Duration, t4: Duration) -> f64 {
    ((t2.as_secs_f64() - t1.as_secs_f64()) + (t3.as_secs_f64() - t4.as_secs_f64())) / 2.0
}

fn rtt(t1: Duration, t2: Duration, t3: Duration, t4: Duration) -> Duration {
    let client_elapsed = t4.saturating_sub(t1);
    let server_elapsed = t3.saturating_sub(t2);
    client_elapsed.saturating_sub(server_elapsed)
}
```

Bu örnek formülü gösterir; `offset_seconds` için T1–T4 zamanlarının aynı saat ölçeğinde karşılaştırılabilmesi gerekir. Gerçek NTP paketlerinde T2 ve T3 sunucunun NTP timestamp alanlarıdır. T1 ve T4 istemci saatinden alınır. İstemci-sunucu saat farkını çıkarmak için sunucu ve istemci saatleri aynı referansa bağlanmış veya birbirine karşı NTP ölçümü yapabilmelidir.

### Hangi durumda formül geçerli?

- İstemci ve sunucu aynı gerçek zaman referansına yakınsa ve yol simetrik kabul edilirse, yukarıdaki formül offset tahmini verir.
- `T1/T4` için istemcide monotonic saat kullanılabilir; `T2/T3` için duvar saati/NTP timestamp kullanılır. Monotonic başlangıç noktası iki tarafta aynı değildir; bu nedenle T2/T3’ü monotonic süre sanmak doğru değildir.
- Gerçek bir ağ saat ölçümünde NTP timestamp epoch/fraction dönüşümleri, 64-bit timestamp ve gerekiyorsa 2036 era geçişi doğru yapılmalıdır.
- Uzak masaüstünde yalnızca “uzak makinanın `now` değeri” alınıp bunu istemcideki `now` ile karşılaştırmak tek başına yeterli değildir; ağ gecikmesi ile saat farkı aynı gözlemde ayrılmaz.
- Tek bir gidiş-dönüş yerine birden fazla ölçüm toplanmalı, düşük RTT ölçümleri tercih edilmeli, medyan/ortalama ve jitter filtrelenmelidir.

### Gecikmenin offset hatasına etkisi

`d1` istemciden sunucuya gidiş, `d2` sunucudan istemciye dönüş gecikmesi olsun. Basit varsayım altında hesaplanan offset şu biçimde yanılabilir:

```text
ölçülen_offset = gerçek_offset + (d1 - d2) / 2
```

Yani dönüş yolu gidiş yolundan farklıysa saat farkı ölçümü kaynaklı bias içerir. VPN, uplink/downlink farkı, Wi-Fi, router kuyrukları, sanal makine pause/stop, CPU scheduling, NTP server işlem yükü ve paket serileştirme bu asimetriyi artırabilir.

### Bilinen hatalar

- **Asimetrik yol:** `(d1 - d2) / 2` hatası en klasik yanlış ölçüm nedenidir.
- **Saat ayarı ölçüm sırasında değişirse:** T1–T4 artık aynı saat epoch’unda değildir; sonuç kararsız olur.
- **NTP server’ın kendi hatası:** İstemci, NTP server’ın UTC’ye olan hatasını da ölçer. Sunucu stratum/root delay/root dispersion/jitter değerleri hesaba katılmazsa “doğru UTC” varsayımı yapılamaz.
- **Ağ gecikmesi hesaba katılmadan:** Uzak masaüstünde `remote_now` gönderme anı ile alınma anı arasındaki mesaj gecikmesi saat farkı sanılabilir.
- **Düşük çözünürlüklü veya hatalı wall clock:** Windows’un saat düzeltmesi, VM snapshot’ı, otomatik zaman servisi veyaRTC sıçraması ölçümü bozabilir.
- **Epoch/era hatası:** NTP timestamp epoch’u 1900’dir; Unix epoch’a çevirmeden saniyelik fark hesaplamak 2036 era geçişinde sorun çıkarabilir.
- **Tek yönlü güvenlik varsayımı:** Kimlik doğrulaması olmayan NTP yanıtı spoof edilebilir. Güvenilir zaman ve replay koruması gerekiyorsa RFC 8915 NTS veya eşdeğer doğrulanmış zaman kaynağı kullanılmalıdır.
- **Güvenilir transport yanılgısı:** TCP üzerinde NTP retry yapılması gecikmeyi artırarak basit offset hesabını bozabilir; NTP normalde UDP kullanır ve tek seferlik returnable-time yaklaşımıyla çalışır.
- **Yerel monotonic saat karıştırma:** `Instant` ve NTP wall timestamp aynı zaman tabanı değildir.

### Uzak masaüstü tasarımı için öneri

1. Önce sistem saatlerini güvenilir NTP/NTS veya işletim sistemi zaman hizmetiyle senkronize et.
2. Her gidiş-dönüşte T1–T4’ü ayrı zaman tabanlarıyla kaydet.
3. RTT ve en küçük gecikme ölçümlerini logla; tek ölçümü nihai saat kabul etme.
4. Uzak masaüstü komutlarında gönderici zamanını karşı tarafın saat farkı düzeltilmiş zamanı gibi kullanma; gecikme bütçesi ve sequence number ile çalış.
5. Güvenlik gerekiyorsa NTP’ye güvenmeden, imzalı/kanonik bir zaman veya otomatik saat senkronizasyonu sağlayan transport tasarla.
6. VM askıya alma, Windows time service, NTP server değişimi ve saat sıçramaları için yeniden ölçüm tetikle.

### DENETIM ICIN KONTROL:

- T1/T4 istemci monotonic veya duvar saatiyle, T2/T3 sunucu NTP timestamp’iyle doğru mu eşleştiriliyor?
- RTT `(T4-T1)-(T3-T2)` yerine yanlışlıkla yalnızca `T4-T1` kullanılmıyor mu?
- `offset` formülünde yol asimetrisi için bias, jitter ve tek ölçüm riski ölçülüyor mu?
- NTP server stratum/root dispersion ve güvenlik kimlik doğrulaması hesaba katılıyor mu?
- Saat servisi, VM resume/suspend ve Windows clock değişiminden sonra yeniden ölçüm yapılıyor mu?

## Kaynak özeti

- [`windows-capture` crates.io](https://crates.io/crates/windows-capture/2.0.1) ve [2.0.1 docs.rs](https://docs.rs/windows-capture/2.0.1/windows_capture/)
- [`windows-capture` GitHub README ve kaynak kodu](https://github.com/NiiightmareXD/windows-capture)
- [Microsoft Screen capture](https://learn.microsoft.com/en-us/windows/apps/develop/media-authoring-processing/screen-capture)
- [Microsoft `GraphicsCaptureSession.IsBorderRequired`](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.graphicscapturesession.isborderrequired)
- [Microsoft `GraphicsCaptureSession.IsCursorCaptureEnabled`](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.graphicscapturesession.iscursorcaptureenabled)
- [`vigem-client` crates.io](https://crates.io/crates/vigem-client/0.1.4) ve [0.1.4 docs.rs](https://docs.rs/vigem-client/0.1.4/vigem_client/)
- [`vigem-client` kaynak kodu ve notification örneği](https://github.com/CasualX/vigem-client)
- [`gilrs` crates.io](https://crates.io/crates/gilrs/0.11.2) ve [0.11.2 docs.rs](https://docs.rs/gilrs/0.11.2/gilrs/)
- [`gilrs` `Gilrs` event loop](https://docs.rs/gilrs/0.11.2/gilrs/struct.Gilrs.html) ve [`Axis`](https://docs.rs/gilrs/0.11.2/gilrs/ev/enum.Axis.html)
- [`quinn` crates.io](https://crates.io/crates/quinn/0.11.12) ve [0.11.12 docs.rs](https://docs.rs/quinn/0.11.12/quinn/)
- [`quinn::Connection`](https://docs.rs/quinn/0.11.12/quinn/struct.Connection.html), [`TransportConfig`](https://docs.rs/quinn/0.11.12/quinn/struct.TransportConfig.html) ve [`SendDatagramError`](https://docs.rs/quinn/0.11.12/quinn/enum.SendDatagramError.html)
- [RFC 5905](https://www.rfc-editor.org/rfc/rfc5905) ve [RFC 8915](https://www.rfc-editor.org/rfc/rfc8915)
