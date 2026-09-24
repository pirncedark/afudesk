//! Dart'a açılan AfuDesk API'si. Uygulamada aynı anda bir host ve bir izleyici oturumu olur.
//! Olaylar düz DTO'larla (tur alanı) akar; freezed/build_runner gerekmez.
use crate::frb_generated::StreamSink;
use afudesk_core::{
    host::{self, HostAyar, HostKomut, HostOlay},
    izleyici::{self, IzleyiciOlay},
    protokol::{FareTusu, Girdi, Izinler},
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};

fn rt() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .thread_name("afudesk")
            .build()
            .expect("tokio")
    })
}

/// Çalışan host ve komut göndericisi (olay alıcısı pompaya taşınır; kilit tutulmaz).
static HOST: Mutex<Option<(host::Host, tokio::sync::mpsc::Sender<HostKomut>)>> = Mutex::new(None);
static IZLEYICI: Mutex<Option<Arc<izleyici::Izleyici>>> = Mutex::new(None);
/// Dart son kareyi çizdi mi? (en-yeni-kazanır geri basıncı)
static KARE_SERBEST: AtomicBool = AtomicBool::new(true);

/// Host olayı. `tur`: hazir | istek | baglandi | koptu | hata.
#[derive(Debug, Clone, Default)]
pub struct HostOlayi {
    pub tur: String,
    pub kod: String,
    pub parola: String,
    pub erisim: String,
    pub adresler: Vec<String>,
    pub ad: String,
    pub metin: String,
    pub kontrol: bool,
}

/// İzleyici olayı. `tur`: bekliyor | kabul | kare | istatistik | koptu | hata.
#[derive(Debug, Clone, Default)]
pub struct IzleyiciOlayi {
    pub tur: String,
    pub rtt_ms: u32,
    pub fps: u32,
    pub ad: String,
    pub metin: String,
    pub kontrol: bool,
    pub genislik: u32,
    pub yukseklik: u32,
    pub rgba: Vec<u8>,
}

/// Girdi. `tur`: konum | fare | kaydir | tus | metin.
#[derive(Debug, Clone, Default)]
pub struct GirdiOlayi {
    pub tur: String,
    pub x: f64,
    pub y: f64,
    /// fare: sol | sag | orta; tus: tuş adı.
    pub ad: String,
    pub basili: bool,
    pub dx: i32,
    pub dy: i32,
    pub metin: String,
}

fn host_dto(o: HostOlay) -> HostOlayi {
    match o {
        HostOlay::Hazir { kod, parola, adresler, erisim } => {
            HostOlayi { tur: "hazir".into(), kod, parola, adresler, erisim, ..Default::default() }
        }
        HostOlay::Istek { ad } => HostOlayi { tur: "istek".into(), ad, ..Default::default() },
        HostOlay::Baglandi { ad, izinler } => {
            HostOlayi { tur: "baglandi".into(), ad, kontrol: izinler.kontrol, ..Default::default() }
        }
        HostOlay::Koptu { sebep } => HostOlayi { tur: "koptu".into(), metin: sebep, ..Default::default() },
        HostOlay::Hata(m) => HostOlayi { tur: "hata".into(), metin: m, ..Default::default() },
    }
}

/// Bu bilgisayarın görünen adı (karşı tarafa gösterilir).
#[flutter_rust_bridge::frb(sync)]
pub fn cihaz_adi() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "AfuDesk".into())
}

#[flutter_rust_bridge::frb(sync)]
pub fn surum() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// Bağlantı ver: sunucuyu açar, olayları `olaylar` akışına yazar. Önceki host varsa durdurulur.
pub fn host_baslat(ad: String, parola: String, upnp: bool, olaylar: StreamSink<HostOlayi>) {
    host_durdur();
    let hata = |olaylar: &StreamSink<HostOlayi>, m: String| {
        let _ = olaylar.add(HostOlayi { tur: "hata".into(), metin: m, ..Default::default() });
    };
    #[cfg(target_os = "android")]
    {
        let _ = (ad, parola, upnp);
        hata(&olaylar, "Bu cihazdan bağlantı verme henüz desteklenmiyor; yalnız bağlanabilirsin.".into());
        return;
    }
    #[cfg(not(target_os = "android"))]
    let fab: Arc<dyn afudesk_core::platform::Fabrika> = Arc::new(afudesk_core::platform::masaustu::Gercek);
    #[cfg(not(target_os = "android"))]
    {
    let mut h = match rt().block_on(host::baslat(HostAyar { ad, port: 0, upnp, parola, yalniz_yerel: false }, fab)) {
        Ok(h) => h,
        Err(e) => return hata(&olaylar, format!("Bağlantı açılamadı: {e}")),
    };
    let mut alici = std::mem::replace(&mut h.olaylar, tokio::sync::mpsc::channel(1).1);
    let tx = h.komut_gonderici();
    *HOST.lock().unwrap() = Some((h, tx));
    rt().spawn(async move {
        while let Some(o) = alici.recv().await {
            if olaylar.add(host_dto(o)).is_err() {
                break;
            }
        }
    });
    }
}

fn host_komut(k: HostKomut) {
    let tx = HOST.lock().unwrap().as_ref().map(|(_, t)| t.clone());
    if let Some(tx) = tx {
        rt().spawn(async move {
            let _ = tx.send(k).await;
        });
    }
}

pub fn host_kabul(kontrol: bool) {
    host_komut(HostKomut::Kabul(Izinler { kontrol, pano: false }));
}

pub fn host_red() {
    host_komut(HostKomut::Red);
}

pub fn host_kes() {
    host_komut(HostKomut::Kes);
}

pub fn host_durdur() {
    // Host düşürülünce Drop durdurma sinyalini gönderir.
    if let Some((mut h, _)) = HOST.lock().unwrap().take() {
        h.durdur();
    }
}

/// Koda bağlan. Her şey (hatalar dahil: `tur = "hata"`) `olaylar` akışından gelir.
/// Not: FRB akış fonksiyonunun Err dönüşü akışa değil yakalanmamış istisnaya gider;
/// bu yüzden hata olay olarak yazılır.
pub fn izleyici_baglan(kod: String, parola: String, ad: String, olaylar: StreamSink<IzleyiciOlayi>) {
    izleyici_kapat();
    let mut iz = match rt().block_on(izleyici::baglan(&kod, &parola, &ad)) {
        Ok(i) => i,
        Err(e) => {
            let _ = olaylar.add(IzleyiciOlayi { tur: "hata".into(), metin: e.to_string(), ..Default::default() });
            return;
        }
    };
    let mut alici = std::mem::replace(&mut iz.olaylar, tokio::sync::mpsc::channel(1).1);
    let iz = Arc::new(iz);
    *IZLEYICI.lock().unwrap() = Some(iz);
    KARE_SERBEST.store(true, Ordering::SeqCst);
    rt().spawn(async move {
        while let Some(o) = alici.recv().await {
            let dto = match o {
                IzleyiciOlay::OnayBekleniyor { karsi_ad } => IzleyiciOlayi { tur: "bekliyor".into(), ad: karsi_ad, ..Default::default() },
                IzleyiciOlay::Kabul { izinler, genislik, yukseklik } => {
                    IzleyiciOlayi { tur: "kabul".into(), kontrol: izinler.kontrol, genislik, yukseklik, ..Default::default() }
                }
                IzleyiciOlay::Kare { genislik, yukseklik, rgba } => {
                    // Dart önceki kareyi çizmediyse bunu atla; sıradaki daha yeni olacak.
                    if !KARE_SERBEST.swap(false, Ordering::SeqCst) {
                        continue;
                    }
                    IzleyiciOlayi { tur: "kare".into(), genislik, yukseklik, rgba, ..Default::default() }
                }
                IzleyiciOlay::Istatistik { rtt_ms, fps } => {
                    IzleyiciOlayi { tur: "istatistik".into(), rtt_ms, fps, ..Default::default() }
                }
                IzleyiciOlay::Koptu { sebep } => IzleyiciOlayi { tur: "koptu".into(), metin: sebep, ..Default::default() },
            };
            if olaylar.add(dto).is_err() {
                break;
            }
        }
    });
}

/// Dart kareyi ekrana çizdiğinde çağırır.
#[flutter_rust_bridge::frb(sync)]
pub fn kare_cizildi() {
    KARE_SERBEST.store(true, Ordering::SeqCst);
}

pub fn izleyici_girdi(g: GirdiOlayi) {
    let Some(iz) = IZLEYICI.lock().unwrap().clone() else { return };
    let girdi = match g.tur.as_str() {
        "konum" => Girdi::FareKonum { x: g.x as f32, y: g.y as f32 },
        "fare" => Girdi::FareTus {
            tus: match g.ad.as_str() {
                "sag" => FareTusu::Sag,
                "orta" => FareTusu::Orta,
                _ => FareTusu::Sol,
            },
            basili: g.basili,
        },
        "kaydir" => Girdi::Kaydir { dx: g.dx, dy: g.dy },
        "tus" => Girdi::Tus { ad: g.ad, basili: g.basili },
        "metin" => Girdi::Metin(g.metin),
        _ => return,
    };
    rt().spawn(async move { iz.gonder(girdi).await });
}

pub fn izleyici_kapat() {
    if let Some(iz) = IZLEYICI.lock().unwrap().take() {
        iz.kapat();
    }
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}
