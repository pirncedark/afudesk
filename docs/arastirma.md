# AfuDesk için Rust + Flutter uzak masaüstü araştırması

**Araştırma tarihi:** 24 Eylül 2026  
**Kapsam:** Windows 10/11 x64 host + viewer, sonraki aşama Android viewer.  
**Not:** Sürüm ve tarih bilgileri kaynak sayfalarında 24 Eylül 2026 itibarıyla kontrol edilmiştir. GitHub API'deki `pushed_at`, son commit tarihi için yaklaşık üst sınırdır; crates/pub.dev tarihleri paket yayın tarihidir. “Doğrulanmadı” ifadesi, kaynak veya çalıştırılmış bir örnek bulunamadığını belirtir.

## 1. Flutter + Rust açık kaynak uzak masaüstü / ekran paylaşım projeleri

### Araştırma sonucu

RustDesk dışında, aynı uygulamada hem Rust çekirdek hem Flutter arayüz kullanan ve özellikle Windows masaüstü host + viewer hedefini karşılayan olgun bir proje doğrulanamadı. En yakın örnekler birbirine komşu katmanları ayırıyor:

| Proje | Son sürüm / durum | Son commit | Lisans | Kısa örnek |
|---|---|---:|---|---|
| [vicajilau/nib](https://github.com/vicajilau/nib) | Sürüm/tag doğrulanamadı; Android tarafı aktif, iOS decoder tamamlanmamış | 2026-09-09 | GPL-3.0 | Rust/GTK4 host, Flutter Android client, H.264 + TCP input/video ayrı socketleri; `flutter run` ve `flutter build apk --release` README'de var. |
| [xiachufang/motif](https://github.com/xiachufang/motif) | Sürüm doğrulanamadı; Windows build ve embedded server mevcut | 2026-09-21 | Lisans alanı doğrulanamadı | `crates/motif-server` Rust server; `apps/flutter` Flutter Windows/Android client. `motifd` screen-capture özelliği `--features screen-capture` ile etkinleşiyor. |
| [AusAgentSmith-org/androidconnect-rs](https://github.com/AusAgentSmith-org/androidconnect-rs) | Sürüm doğrulanmadı; MVP 1, encrypted transport yok | 2026-05-25 | GitHub SPDX `NOASSERTION`; README `MIT OR Apache-2.0` diyor, uyuşmuyor | `crates/protocol` + JNI; Android `MediaProjection`/`MediaCodec`; desktop viewer Rust/GPUI ve OpenH264. Windows viewer doğrulanmadı. |
| [nchapman/tether](https://github.com/nchapman/tether) | Pre-MVP; Windows host/client kodu belirtilmiş | 2026-06-11 | MIT | `tether-host` capture/encode/input, `tether-client` decode/render, `tether-transport` QUIC. Flutter yok. |
| [eagleos/eagledesk](https://github.com/eagleos/eagledesk) | README'de Flutter; Rust çekirdeği doğrulanamadı | Doğrulanamadı | Lisans doğrulanamadı | README mimarisi WebSocket/TCP/UDP, H.264 ve platform plugin'leri anlatıyor; gerçek Rust kullanımı için kod incelemesi gerekir. |

**Kısa örnek:** Nib'in mimarisi AfuDesk için şu ayrımı gösteriyor: host tarafında Rust `GStreamer`/H.264 ve input, viewer tarafında Flutter `MediaCodec`/`SurfaceTexture`. Bu, aynı Flutter uygulamasının hem host hem viewer olması anlamına gelmiyor.

**Değerlendirme:** AfuDesk'in fikri olarak RustDesk kodu kullanmadan faydalanabileceği güvenilir referanslar Tether ve AndroidConnect RS'dir. Nib yalnızca Android viewer/host ayrımı, Motif ise Rust + Flutter paketleme ve gömülü server fikri için yararlıdır. Hiçbiri doğrudan “Windows Flutter host + Flutter viewer” referans implementasyonu değildir.

**ONERI:** AfuDesk'in ilk dikey dilimini Windows 10/11 x64 olarak tasarlayıp protokolü, codec'i ve input olaylarını bağımsız crate'lerde yazmak; Tether'den QUIC/video akış ayrımı, AndroidConnect'ten capture/decoder ayrımı, Nib'ten client decode yaklaşımı alınmalı, ancak hiçbir projedeki kod kopyalanmamalı.

## 2. Rust karelerini Flutter Windows'ta gösterme

### Karşılaştırma

| Yöntem | Son sürüm | Son commit | Lisans | 1080p@30 için değerlendirme |
|---|---:|---:|---|---|
| [irondash_texture](https://crates.io/crates/irondash_texture) / [GitHub](https://github.com/irondash/irondash) | 0.5.0 (crates) | 2026-08-07 | MIT | En uygun aday. Rust payload provider Flutter external texture'a bağlanır; `BoxedPixelData` veya Windows `DxgiSharedHandle` kullanılabilir. En az Dart'a RGBA kopyası. |
| [texture_rgba_renderer](https://pub.dev/packages/texture_rgba_renderer) / [GitHub](https://github.com/rustdesk-org/flutter_texture_rgba_renderer) | 0.0.16 | 2026-08-13 | Apache-2.0 | BGRA texture, OpenGL hedefli. README'de Method Channel yaklaşık 45 FPS, Native API yaklaşık 670 FPS gösteriyor; bu üretici ölçümü, sistem performansı garantisi değil. |
| flutter_rust_bridge + `Uint8List` + `RawImage` | 2.13.0 | 2026-09-13 | MIT | En kolay prototip, fakat 1920×1080×4 yaklaşık 8.3 MB kare başına Dart/FFI ve `RawImage` yolu; 30 FPS için pahalı ve takılma riski yüksek. |
| `ui.decodeImageFromPixels` | Flutter SDK API'si; paket sürümü yok | Flutter SDK'ya bağlı | Flutter/BSD tarafı | Ham pikseli `ui.Image` yapar; decode edilen image ve `RawImage` maliyetini taşır. Sık 30 FPS akış için ilk tercih değil. |
| [irondash_texture](https://docs.rs/irondash_texture/latest/irondash_texture/struct.Texture.html) | 0.5.0 | 2026-08-07 | MIT | Windows'ta `BoxedTextureDescriptor<DxgiSharedHandle>` mümkün; doğrudan GPU payload, BGRA CPU kopyasından daha iyi. Engine handle ve texture oluşturma platform thread kuralına dikkat edilmeli. |

### Kurulum adımları

1. Flutter Windows uygulaması oluşturulur ve desktop hedefi etkinleştirilir.
2. FRB v2 kurulur; aynı sürümün Dart paketi ve `flutter_rust_bridge_codegen` binary'si kullanılır.
3. Rust tarafında `irondash_texture` ve gerekiyorsa `irondash_engine_context` eklenir.
4. Texture Flutter engine handle ile oluşturulur; Rust provider `mark_frame_available()` çağırır.
5. Dönen texture ID `Texture(textureId: id)` ile gösterilir.
6. GPU payload mümkünse DXGI shared handle; değilse `BoxedPixelData` kullanılır. `BoxedPixelData` için kanal sırası ve stride platforma göre doğrulanmalıdır.
7. FRB `StreamSink<Frame>` yalnızca kontrol/veri taşıma için kullanılmalı; her kare için yeni `Uint8List` ve `RawImage` yerine texture ID sabit tutulmalıdır.

**Kısa Rust örneği (irondash_texture şeması):**

```rust
use std::sync::Arc;
use irondash_texture::{PayloadProvider, SimplePixelData, Texture};

struct Provider;

impl PayloadProvider<BoxedPixelData> for Provider {
    fn get_payload(&self) -> BoxedPixelData {
        SimplePixelData::boxed(1920, 1080, vec![0; 1920 * 1080 * 4])
    }
}

fn create(engine_handle: i64) -> Result<Texture<BoxedPixelData>, Box<dyn std::error::Error>> {
    let texture = Texture::new_with_provider(engine_handle, Arc::new(Provider))?;
    Ok(texture)
}
```

**Kısa Dart örneği:**

```dart
final id = await createNativeTexture(engineHandle, 1920, 1080);

Texture(
  textureId: id,
  width: 1920,
  height: 1080,
)
```

`createNativeTexture` isimli uygulama fonksiyonu ve texture ID dönüşü, seçilen `irondash_texture` sürümüne göre doğrulanmalıdır; yukarıdaki Flutter widget parçası fikir düzeyindedir.

**ONERI:** AfuDesk için `texture_rgba_renderer` bir konsept/fallback olarak, ana yol olarak `irondash_texture` ve özellikle Windows DXGI shared handle seçilmeli. `RawImage` yalnızca düşük çözünürlükte debug/uygulama içi test için kullanılmalı.

## 3. flutter_rust_bridge v2 Windows kurulumu ve StreamSink

**Kaynaklar:** [GitHub](https://github.com/fzyzcjy/flutter_rust_bridge), [pub.dev](https://pub.dev/packages/flutter_rust_bridge), [quickstart](https://github.com/fzyzcjy/flutter_rust_bridge/blob/master/website/docs/quickstart.md), [Stream dokümanı](https://cjycode.com/flutter_rust_bridge/guides/types/translatable/stream).

- Dart paketi ve Rust runtime: **2.13.0**, 23 Ağustos 2026.
- Codegen: **2.13.0**; aynı sürümü sabitlemek en güvenli yol.
- Son commit: 13 Eylül 2026.
- Lisans: MIT.
- Windows için CargoKit entegrasyonu destekleniyor. Native Assets backend yalnızca `2.13.0-beta.2` veya üzeri ile, build hooks/code assets destekleyen Flutter/Dart SDK'da kullanılabiliyor.

### Kurulum

```text
flutter pub add flutter_rust_bridge:2.13.0
cargo install flutter_rust_bridge_codegen --version 2.13.0
```

Yeni uygulama:

```text
flutter_rust_bridge_codegen create afudesk
```

Mevcut Flutter projesinin kökünde:

```text
flutter_rust_bridge_codegen integrate
```

Rust API değiştiğinde:

```text
flutter_rust_bridge_codegen generate
```

Native Assets denemek istenirse:

```text
flutter_rust_bridge_codegen integrate --integration-backend native-assets
```

CargoKit, `integrate` sırasında Windows CMake/Flutter build zincirine eklenir. `build.rs` veya platforma özel DLL kopyalama ayrıntıları generated template'e bırakılmalı; sürüm farklarında resmi quickstart kontrol edilmelidir.

### StreamSink

FRB stream'i Rust tarafında `StreamSink<T>` ile üretir; Dart tarafında `Stream<T>` olur. Stream, Rust fonksiyonu döndükten sonra da yaşatılabilir. Her kareyi bu stream'e koymak yerine sadece metadata ve kontrol olaylarını göndermek daha sağlıklıdır; kare GPU/ttexture yolundan gitmelidir.

```rust
use crate::frb_generated::StreamSink;

pub fn watch_session(sink: StreamSink<String>) {
    sink.add("connected".to_owned());
}
```

```dart
final events = watchSession();

events.listen((event) {
  print(event);
});
```

**ONERI:** FRB'yi ilk sürümde Dart uygulaması ile Rust çekirdeği arasındaki kontrol düzlemi ve düşük frekanslı olay akışı için kullanmak; 30 FPS ham kare akışını FRB StreamSink'e bırakmamak. `StreamSink` kare taşıyacaksa en fazla metadata + sıkıştırılmış paket sınırı koyulmalı ve backpressure uygulanmalı.

## 4. Rust Windows ekran yakalama

| Kütüphane | Son sürüm | Son commit | Lisans | API / FPS notu |
|---|---:|---:|---|---|
| [xcap](https://crates.io/crates/xcap) / [GitHub](https://github.com/nashaofu/xcap) | 0.9.7 | 2026-08-25 | Apache-2.0 | `Monitor::capture_image`, `capture_region`, `video_recorder`; screenshot için kolay, sürekli stream için backend ayrıntısı incelenmeli. |
| [windows-capture](https://crates.io/crates/windows-capture) / [GitHub](https://github.com/NiiightmareXD/windows-capture) | 2.0.1 | 2026-09-18 | MIT | Windows Graphics Capture + DXGI Desktop Duplication; `on_frame_arrived`, dirty-region ayarları ve donanım hızlandırmalı encoder mevcut. |
| [scap](https://crates.io/crates/scap) / [GitHub](https://github.com/CapSoftware/scap) | 0.1.0-beta.1 | 2025-08-04 | MIT | `Options { fps, output_type: BGRAFrame, ... }`; cross-platform ve permissions için uygun, beta. |
| [win_desktop_duplication](https://crates.io/crates/win_desktop_duplication) / [GitHub](https://github.com/rhinostream/win_desktop_duplication) | 0.10.11 | 2026-08-19 | MIT | Düşük seviye DXGI; vsync başına frame, `TextureReader` ile CPU kopyası. GPU encoder'a doğrudan vermek daha verimli. |

`xcap` 0.9.x, `windows-capture` 2.x ve `scap` 0.1 beta farklı problemleri çözüyor. 30 FPS sabit bir garanti değildir; Windows ayarları, monitör refresh rate, cursor hareketi, GPU ve encoder kapasitesi ölçülmelidir. `windows-capture` README'sinde varsayılan minimum update aralığının 60 FPS/16.67 ms olabileceği ve bunun garanti olmadığı belirtiliyor.

**Kısa örnek (`windows-capture` DXGI):**

```rust
use windows_capture::dxgi_duplication_api::DxgiDuplicationApi;
use windows_capture::monitor::Monitor;

fn grab() -> Result<(), Box<dyn std::error::Error>> {
    let monitor = Monitor::primary()?;
    let mut duplication = DxgiDuplicationApi::new(monitor)?;
    let frame = duplication.acquire_next_frame(33)?;
    println!("{}x{}", frame.width(), frame.height());
    Ok(())
}
```

**ONERI:** Windows host için `windows-capture 2.0.1` ana yakalama yolu, `xcap 0.9.7` CLI/uygulama içi screenshot fallback'i olarak seçilmeli. Sürekli akışta GPU kopyasını korumak için `DxgiSharedHandle` benzeri bir çıktı hedeflenmeli.

## 5. H.264/VP8 kodlama-çözme ve JPEG + dirty tile

### H.264

[openh264](https://crates.io/crates/openh264), [GitHub](https://github.com/ralfbiedert/openh264-rs):

- Son sürüm: **0.9.7**, 8 Temmuz 2026.
- Son commit: doğrudan crate sayfasından ayrı commit tarihi doğrulanamadı; crates güncellemesi 2026-07-08.
- Lisans: wrapper ve Cisco OpenH264 core için BSD-2-Clause; kaynak README'de doğrulandı.
- Decoder: `Decoder::decode(packet)`, encoder: `Encoder::encode(&yuv)`.
- Windows MSVC binary olarak test edilmiş; varsayılan özellik Cisco kaynak kodunu derler, `libloading` ile ayrı binary sağlanabilir.
- Salt kodla yazılan Rust C API güvenlik garantisi vermez; upstream C decoder'ı da saldırı yüzeyidir.

**Kısa örnek:**

```rust
use openh264::{decoder::Decoder, encoder::Encoder, nal_units};

fn decode_h264(bytes: &[u8]) -> Result<Vec<openh264::decoder::Yuv>, Box<dyn std::error::Error>> {
    let mut decoder = Decoder::new()?;
    Ok(nal_units(bytes).filter_map(|packet| decoder.decode(packet).ok()).collect())
}

fn encode_h264(yuv: &[openh264::encoder::Yuv]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut encoder = Encoder::new()?;
    Ok(encoder.encode(yuv)?.into_vec())
}
```

Yukarıdaki yuv tür adları sürüm 0.9.7'nin gerçek tip adlarıyla derlenmeyebilir; resmi README'deki `encode(&yuv)` ve `decode` akışı esas alınmalı, tip adları uygulanmadan önce doğrulanmalıdır.

### Alternatifler

- **VP8/VP9:** WebRTC/FFmpeg/paketleme bağımlılığı ve platform codec lisansları nedeniyle bu araştırmada doğrulanmış saf Rust VP8 codec yığını seçilmedi. H.264/VP8 için codec ve patent/lisans uyumu ayrıca hukuk kontrolü gerektirir.
- **AV1:** `rav1e` saf Rust encoder seçeneği olarak değerlendirilebilir; decoder için platform/FFmpeg kararı gerekir. 30 FPS hedefi için donanım codec'i tercih edilmelidir.
- **Donanım codec:** Windows Media Foundation, D3D11/NVENC/QSV gerçek ekran verisine en yakın yoldur; crate soyutlamasından ayrı Windows adapter gerekir. Bu raporde kesin crate/API doğrulanmadı.

### JPEG + dirty tile

`jpeg-encoder` **0.7.1** (MIT OR Apache-2.0 AND IJG, 27 Temmuz 2026) baseline/progressive JPEG ve SIMD seçenekleri sunuyor. Ancak JPEG'li bir uzak masaüstü için en doğru tasarım, tam kareyi her zaman JPEG'lemek yerine şu protokoldür:

1. Ekranı 64x64 veya 128x128 tile'lara böl.
2. Önceki karele karşılaştır; değişmeyen tile'ı atla.
3. Değişen tile'ları birleştir, JPEG/MJPEG veya ham RGBA ile gönder.
4. Paket başlığına `frame_id`, `x`, `y`, `width`, `height`, `quality`, `codec` koy.
5. İlk kare tam; periyodik olarak referans kare veya tam kare gönder.
6. Kayıp tile varsa son tam kareyi tekrar iste.

Bu yaklaşımın güvenilir bir açık kaynak uzak masaüstü referansı doğrulanmadı; aşağıdaki paket semantiği AfuDesk için tasarlanacak protokoldür.

```rust
struct TilePacket {
    frame_id: u32,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    quality: u8,
    jpeg: Vec<u8>,
}
```

JPEG'nin 4:2:0 subsampling'i, metin ve ince çizgilerde kaliteyi düşürebilir. Dirty tile algoritması bileşik değişikliklerde çok sayıda paket üretir; kare sınırı, coalescing ve toplam bitrate limiti gerekir.

**ONERI:** İlk sürüm H.264 ile başlanmalı; JPEG + dirty tile yalnızca düşük bantlı, düşük çözünürlüklü geri düşüş modu olmalı. `openh264` başlangıç codec'i olarak, `jpeg-encoder` yalnızca fallback seçilmeli.

## 6. quinn 0.11 + rustls + rcgen ve parmak izi sabitleme

| Paket | Son sürüm | Son commit | Lisans |
|---|---:|---:|---|
| [quinn](https://crates.io/crates/quinn) | 0.11.12, 14 Eylül 2026 | 2026-09-23 | MIT OR Apache-2.0 |
| [rustls](https://crates.io/crates/rustls) | 0.23.45 stable, 14 Eylül 2026 | 2026-09-23 | MIT/Apache-2.0 ve ek lisans dosyaları; crate lisans alanında `NOASSERTION`, README/license dosyaları kontrol edilmeli |
| [rcgen](https://crates.io/crates/rcgen) | 0.14.10, 28 Ağustos 2026 | 2026-09-22 | MIT OR Apache-2.0 |

QUIC + rustls için iki ayrı güvenlik katmanı gerekir: TLS şifreleme ve parmak izi sabitleme. Kullanıcı kodu içinde taşınan sabit, üretilen DER sertifikanın SHA-256 parmak izi olmalıdır. İlk bağlantıda güvenilen kanal/QR ile pin alınmalı, sonraki bağlantılarda pin değişirse bağlantı reddedilmelidir. `ServerCertVerifier` imzası rustls 0.23 şu şekildedir: `verify_server_cert`, `verify_tls12_signature`, `verify_tls13_signature`, `supported_verify_schemes`.

### Çalışır kriterlere yakın örnek

```rust
use std::sync::Arc;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::CryptoProvider;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, Error, SignatureScheme};
use sha2::{Digest, Sha256};

#[derive(Debug)]
struct PinVerifier {
    expected: Vec<u8>,
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for PinVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        let actual = Sha256::digest(end_entity.as_ref());
        if actual.as_slice() == self.expected.as_slice() {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(Error::General("certificate pin mismatch".into()))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider.signature_verification_algorithms.supported_schemes()
    }
}
```

`rustls::ClientConfig::builder().dangerous().with_custom_certificate_verifier(Arc::new(PinVerifier { expected, provider })).with_no_client_auth()` ile `quinn::ClientConfig::new(Arc::new(QuicClientConfig::try_from(tls_config)?))` oluşturulur. QUIC istiyorsa provider'ın TLS 1.3 ve QUIC uyumlu algoritmaları seçmesi gerekir.

### rcgen sunucu sertifikası

```rust
use rcgen::{generate_simple_self_signed, CertifiedKey};

let CertifiedKey { cert, key_pair } =
    generate_simple_self_signed(vec!["afudesk.example".to_owned()])?;
let cert_der = cert.der().to_vec();
let key_der = key_pair.serialize_der();
```

Sertifikayı her yeniden başlatmada yeniden üretmek parmak izi değiştirir. Üretilen sertifika kalıcı saklanmalı veya üretim sırasında parmak izi güvenli biçimde alıcıya aktarılmalıdır. `quinn` dokümanı da TOFU için rcgen'in kalıcı anahtar/sertifika ile kullanılmasını önerir.

**ONERI:** `quinn 0.11.12` + `rustls 0.23.45` + `rcgen 0.14.10` ve SHA-256 sertifika parmak izi sabitleme kullanılmalı. QUIC için video datagram, input/keyboard/control ise reliable stream olarak ayrılmalı; kod tabanında `custom ServerCertVerifier` imzası sürüm kilitlendikten sonra compile-test edilmeli.

## 7. igd-next 0.17 ile UPnP

| Paket | Son sürüm | Son commit | Lisans |
|---|---:|---:|---|
| [igd-next](https://crates.io/crates/igd-next) / [GitHub](https://github.com/dariusc93/rust-igd) | 0.17.1, 1 Haziran 2026 | 2026-08-30 | MIT |

`search_gateway()` ile `Gateway` bulunur. `get_external_ip()`, `add_port()`, `add_any_port()` ve `remove_port()` API'leri doğrulandı. Lease `0` kalıcı; router desteklemiyorsa hata oluşabilir. UPnP port açma güvenlik açığı yaratabilir; sadece kullanıcı açıkça etkinleştirdiğinde ve oturum kapanınca kaldırılmalıdır.

### Kısa örnek

```rust
use igd_next::{search_gateway, PortMappingProtocol};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

fn open_udp(local_ip: Ipv4Addr, port: u16) -> Result<u16, Box<dyn std::error::Error>> {
    let gateway = search_gateway()?;
    let local = SocketAddr::new(IpAddr::V4(local_ip), port);
    let external = gateway.add_any_port(PortMappingProtocol::UDP, local, 0, "AfuDesk")?;
    println!("external={}:{}", gateway.get_external_ip()?, external);
    Ok(external)
}
```

Sabit port istendiğinde `add_port(PortMappingProtocol::UDP, port, local, 0, "AfuDesk")`, kapanışta `remove_port(PortMappingProtocol::UDP, external_port)` kullanılır. `add_any_port` bazı modemlerde dış portu değiştirebilir; dönen port kodun içine yazılmalıdır.

**ONERI:** `igd-next 0.17.1` kullanılmalı, ancak IPv6 için UPnP yerine firewall/router durumu ayrı ele alınmalı. UPnP başarısız olursa uygulama LAN/IPv6 adreslerini göstermeye devam etmeli; bağlantı kodunun tek yolunu UPnP'e bağımlı yapmamalı.

## 8. enigo 0.6 ile fare/klavye

| Paket | Son sürüm | Son commit | Lisans |
|---|---:|---:|---|
| [enigo](https://crates.io/crates/enigo) / [GitHub](https://github.com/enigo-rs/enigo) | 0.6.1, 28 Ağustos 2025 | 2026-09-21 | MIT |

### Kısa örnek

```rust
use enigo::{Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};

fn send_input() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = Enigo::new(&Settings::default())?;
    input.move_mouse(500, 200, Coordinate::Abs)?;
    input.button(Button::Left, Direction::Click)?;
    input.scroll(0, -3)?;
    input.key(Key::Control, Direction::Press);
    input.key(Key::Unicode('v'), Direction::Click);
    input.key(Key::Control, Direction::Release);
    input.text("hello")?;
    Ok(())
}
```

Windows'ta UAC ayrı masaüstü ve yüksek bütünlüklü süreçlerde normal kullanıcıdan yöneticiye geçiş gerekebilir. Enigo, Windows `SendInput`/doğrudan yöntemler kullandığı için hedef uygulamanın yönetici seviyesinde çalışması veya host process'inin yönetici olarak başlatılması gerekebilir. Güvenlik açısından yalnızca açıkça onaylanmış oturumda, viewer yetkisi sınırlıyken ve gönderilen tuşları normalize ederek kullanılmalıdır. Windows'ta yüksek DPI ve farklı masaüstleri için Settings/dokümantasyon doğrulanmalıdır.

**ONERI:** `enigo 0.6.1` prototip ve platform gerçeklemesi için kullanılabilir; UAC ve secure desktop davranışı MVP kabul kriteri yapılmalı. Android viewer'da host-side injection yok; sadece dokunmatik/klavye olayları hedef platform API'sine çevrilmeli.

## 9. Yerel IP / IPv6 adresleri

| Paket | Son sürüm | Son commit | Lisans |
|---|---:|---:|---|
| [local-ip-address](https://crates.io/crates/local-ip-address) | 0.6.13, 19 Mayıs 2026 | Doğrudan doğrulanamadı | MIT OR Apache-2.0 |
| [if-addrs](https://crates.io/crates/if-addrs) | 0.15.0, 8 Şubat 2026 | Doğrudan doğrulanamadı | MIT OR BSD-3-Clause |

`if-addrs`, Posix ve Windows arayüzlerinden adres listeler. `link-local` özelliği ayrıdır; link-local adresleri (`fe80::/10`, IPv4 link-local) doğrudan genel bağlantı adresi gibi sunulmamalıdır. VPN, sanal adaptör, Docker/WSL ve çoklu ağ adaptörü sonuçları kirletebilir. Global IPv6 adresleri scope ve interface index gerektirebilir; link-local adreslerde zone id eklenmelidir.

**Kısa örnek:**

```rust
fn addresses() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(if_addrs::get_if_addrs()?
        .into_iter()
        .map(|interface| format!("{}: {}", interface.name, interface.addr.ip()))
        .collect())
}
```

`local-ip-address` daha yüksek seviyeli bir kolaylık sağlar; fakat AfuDesk'in IPv6 ve çoklu adaptör ihtiyacı nedeniyle `if-addrs` daha görünür bir temel olarak tercih edilebilir. En iyi yaklaşım: UPnP ile dış adresi, `if-addrs` ile LAN adreslerini, ayrıca işletim sistemi socket'ına bağlanarak route/source IP'yi doğrulamak.

**ONERI:** `if-addrs 0.15.0` temel liste için, `local-ip-address 0.6.13` yalnızca kolaylık/fallback olarak kullanılmalı. Kodda `IsGlobal`, loopback, link-local ve VPN/sanal adaptör elemesi açıkça yapılmalı; IPv6 adresinde scope id korunmalı.

## Kaynak ve doğrulama notları

- [crates.io quinn](https://crates.io/crates/quinn), [rustls](https://crates.io/crates/rustls), [rcgen](https://crates.io/crates/rcgen), [igd-next](https://crates.io/crates/igd-next), [enigo](https://crates.io/crates/enigo), [if-addrs](https://crates.io/crates/if-addrs), [local-ip-address](https://crates.io/crates/local-ip-address), [windows-capture](https://crates.io/crates/windows-capture), [xcap](https://crates.io/crates/xcap), [scap](https://crates.io/crates/scap), [win_desktop_duplication](https://crates.io/crates/win_desktop_duplication), [openh264](https://crates.io/crates/openh264), [jpeg-encoder](https://crates.io/crates/jpeg-encoder), [irondash_texture](https://crates.io/crates/irondash_texture).
- [pub.dev texture_rgba_renderer](https://pub.dev/packages/texture_rgba_renderer), [pub.dev flutter_rust_bridge](https://pub.dev/packages/flutter_rust_bridge).
- [Quinn certificate configuration](https://quinn-rs.github.io/quinn/quinn/certificate.html), [rustls ServerCertVerifier API](https://docs.rs/rustls/latest/rustls/client/danger/trait.ServerCertVerifier.html), [FRB Stream](https://cjycode.com/flutter_rust_bridge/guides/types/translatable/stream).
- GitHub proje son commit tarihleri için repo API'sindeki `pushed_at`; lisans için GitHub SPDX alanı ve repo lisans dosyası birlikte kontrol edilmelidir. Bir repo API'si lisans alanı `NOASSERTION` veya boş döndürürse raporda bu belirsizlik korunmuştur.

## SECILEN YIGIN

| katman | crate/paket | surum | neden |
|---|---|---:|---|
| Flutter/Rust köprüsü | flutter_rust_bridge + codegen | 2.13.0 | Windows dahil Flutter/Rust entegrasyonu, async ve StreamSink desteği; kare pixel'ı FRB'den taşımak yerine kontrol düzlemi için kullanılacak. |
| Flutter kare gösterimi | irondash_texture | 0.5.0 | External texture ve Windows GPU payload yolu; RawImage/UI decode'dan daha uygun. |
| Alternatif texture | texture_rgba_renderer | 0.0.16 | BGRA/OpenGL ve düşük seviye native API; geri düşüş/fallback. |
| Windows yakalama | windows-capture | 2.0.1 | Graphics Capture, DXGI Desktop Duplication, dirty region ve stream callback içeriyor. |
| Screenshot fallback | xcap | 0.9.7 | Monitor/window/region capture için kolay cross-platform API. |
| Düşük seviye capture | win_desktop_duplication | 0.10.11 | GPU frame erişimi istenirse doğrudan DXGI yolu. |
| H.264 | openh264 | 0.9.7 | Windows'ta doğrulanmış H.264 encode/decode ve kaynak/yaşam döngüsü seçenekleri. |
| JPEG fallback | jpeg-encoder | 0.7.1 | Dirty tile JPEG/MJPEG için saf Rust encoder; yalnızca düşük bantlı fallback. |
| QUIC | quinn | 0.11.12 | Async QUIC, datagram ve stream ayrımı; Rust tabanlı uygulama ile uyum. |
| TLS | rustls | 0.23.45 stable | QUIC uyumlu TLS ve custom verifier desteği. |
| Sertifika üretimi | rcgen | 0.14.10 | Kalıcı self-signed sertifika üretimi; parmak izi sabitleme ile birlikte kullanılacak. |
| UPnP | igd-next | 0.17.1 | Gateway arama, external IP ve UDP port mapping. |
| Input injection | enigo | 0.6.1 | Windows/macOS/Linux için kapsamlı klavye ve fare API'si; UAC/protokol sınırları açıkça ele alınmalı. |
| Ağ arayüzleri | if-addrs | 0.15.0 | IPv4/IPv6 ve çoklu adaptörleri görünür kılar; global/link-local filtreleri AfuDesk'te yapılacak. |
| Kolay adres seçimi | local-ip-address | 0.6.13 | Basit kullanıcı adresi fallback'i; temel doğrulama için if-addrs yerine kullanılmamalı. |
