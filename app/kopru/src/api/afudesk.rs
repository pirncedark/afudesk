//! Dart'a açılan AfuDesk API'si. Uygulamada aynı anda bir host ve bir izleyici oturumu olur.
//! Olaylar düz DTO'larla (tur alanı) akar; freezed/build_runner gerekmez.
use crate::frb_generated::StreamSink;
use afudesk_core::{
    guvenilen,
    host::{self, HostAyar, HostKomut, HostOlay},
    izleyici::{self, IzleyiciOlay},
    protokol::{FareTusu, Girdi, Izinler, Kontrol},
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex, OnceLock,
    },
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
static KARE_UYANDIR: tokio::sync::Notify = tokio::sync::Notify::const_new();
static IZLEYICI_NESLI: AtomicU32 = AtomicU32::new(0);
#[derive(Default)]
struct BekleyenParcalar {
    gen: u32,
    yuk: u32,
    tam: bool,
    girdi_ms: u64,
    host_girdi_ms: u64,
    tiles: std::collections::BTreeMap<(u32, u32), afudesk_core::goruntu::DosemeRgba>,
}
impl BekleyenParcalar {
    fn ekle_kare(&mut self, gen: u32, yuk: u32, rgba: Vec<u8>) {
        let mut tiles = Vec::new();
        let adim = afudesk_core::goruntu::DOSEME;
        for y in (0..yuk).step_by(adim as usize) {
            for x in (0..gen).step_by(adim as usize) {
                let w = adim.min(gen - x);
                let h = adim.min(yuk - y);
                let mut pixels = Vec::with_capacity((w * h * 4) as usize);
                for row in y..y + h {
                    let start = (row as usize * gen as usize + x as usize) * 4;
                    pixels.extend_from_slice(&rgba[start..start + w as usize * 4]);
                }
                tiles.push(afudesk_core::goruntu::DosemeRgba {
                    x, y, gen: w, yuk: h, rgba: pixels,
                });
            }
        }
        self.ekle(gen, yuk, true, tiles, 0, 0);
    }
    fn ekle(
        &mut self,
        gen: u32,
        yuk: u32,
        tam: bool,
        tiles: Vec<afudesk_core::goruntu::DosemeRgba>,
        girdi_ms: u64,
        host_girdi_ms: u64,
    ) {
        if tam || self.gen != gen || self.yuk != yuk {
            self.tiles.clear();
            self.tam = true;
        }
        self.gen = gen;
        self.yuk = yuk;
        self.girdi_ms = self.girdi_ms.max(girdi_ms);
        self.host_girdi_ms = self.host_girdi_ms.max(host_girdi_ms);
        for t in tiles {
            self.tiles.insert((t.x, t.y), t);
        }
    }
    fn al(&mut self) -> Option<IzleyiciOlayi> {
        if self.tiles.is_empty() {
            return None;
        }
        let mut rgba = Vec::new();
        for (_, t) in std::mem::take(&mut self.tiles) {
            for n in [t.x, t.y, t.gen, t.yuk] {
                rgba.extend_from_slice(&n.to_le_bytes());
            }
            rgba.extend_from_slice(&t.rgba);
        }
        let dto = IzleyiciOlayi { tur: "parcalar".into(), genislik: self.gen, yukseklik: self.yuk, rgba,
            metin: serde_json::json!({"tam": self.tam,"girdi_ms": self.girdi_ms,"host_girdi_ms": self.host_girdi_ms}).to_string(),
            ..Default::default() };
        self.tam = false;
        self.girdi_ms = 0;
        self.host_girdi_ms = 0;
        Some(dto)
    }
}
static IZLEYICI_KOL_IZNI: AtomicBool = AtomicBool::new(false);
static IZLEYICI_KOL_SIRASI: AtomicU32 = AtomicU32::new(0);
/// Kalıcı cihaz kimliği + kayıtlı cihazlar klasörü (Dart uygulama destek klasörünü verir).
static VERI: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Uygulama açılışında çağrılır: kimlik ve kayıtlı cihazlar bu klasörde saklanır.
#[flutter_rust_bridge::frb(sync)]
pub fn veri_klasoru_ayarla(yol: String) {
    *VERI.lock().unwrap() = Some(PathBuf::from(yol).join("AfuDesk"));
}

pub(super) fn veri() -> Option<PathBuf> {
    VERI.lock()
        .unwrap()
        .clone()
        .or_else(guvenilen::varsayilan_klasor)
}

pub(super) fn host_adresleri() -> Option<(Vec<String>, Vec<String>)> {
    HOST.lock()
        .unwrap()
        .as_ref()
        .map(|(h, _)| h.google_adresleri())
}

pub fn izleyici_google_baglan(kimlik: String, ad: String, olaylar: StreamSink<IzleyiciOlayi>) {
    izleyici_kapat();
    IZLEYICI_KOL_IZNI.store(false, Ordering::SeqCst);
    let sonuc = rt().block_on(async {
        let g = super::google::etkin_hesap()?;
        let v = veri().ok_or_else(|| anyhow::anyhow!("Cihaz kaydedilemedi, yeniden dene."))?;
        #[cfg(not(target_os = "android"))]
        let pano = afudesk_core::pano::ArboardPano::new()
            .ok()
            .map(|p| Box::new(p) as Box<dyn afudesk_core::pano::Pano>);
        #[cfg(target_os = "android")]
        let pano = None;
        izleyici::google_baglan(&g, &kimlik, &v, &ad, pano).await
    });
    izleyici_pompala(sonuc, olaylar);
}

/// Kayıtlı/güvenilen cihaz (listede yalnız ad ve son görülme gösterilir).
#[derive(Debug, Clone, Default)]
pub struct KayitliCihaz {
    /// İç kimlik: yalnız Bağlan/Unut/Kaldır çağrılarında kullanılır, gösterilmez.
    pub kimlik: String,
    pub ad: String,
    pub asil_ad: String,
    pub acik: bool,
    /// Unix saniye.
    pub son_gorulme: i64,
}

static ACIK_KIMLIKLER: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn kayitlar() -> guvenilen::Kayitlar {
    veri()
        .map(|v| guvenilen::oku(&guvenilen::kayit_yolu(&v)))
        .unwrap_or_default()
}

/// Bu cihazın kod olmadan bağlanabileceği bilgisayarlar (en son görülen önce).
#[flutter_rust_bridge::frb(sync)]
pub fn kayitli_cihazlar() -> Vec<KayitliCihaz> {
    let mut v: Vec<_> = kayitlar()
        .izleyiciler
        .into_iter()
        .map(|k| KayitliCihaz {
            acik: ACIK_KIMLIKLER.lock().unwrap().contains(&k.host_kimlik),
            kimlik: k.host_kimlik,
            ad: k.etiket.unwrap_or_else(|| k.ad.clone()),
            asil_ad: k.ad,
            son_gorulme: k.son_gorulme,
        })
        .collect();
    v.sort_by_key(|k| std::cmp::Reverse(k.son_gorulme));
    v
}

/// İzleyici: kaydı unut (karşı tarafa haber verilmez).
#[flutter_rust_bridge::frb(sync)]
pub fn kayitli_unut(kimlik: String) {
    if let Some(v) = veri() {
        let _ = guvenilen::guncelle(&guvenilen::kayit_yolu(&v), |k| {
            guvenilen::hostu_unut(k, &kimlik)
        });
    }
}

/// Bu bilgisayara kod olmadan bağlanabilen cihazlar (en son görülen önce).
#[flutter_rust_bridge::frb(sync)]
pub fn guvenilen_cihazlar() -> Vec<KayitliCihaz> {
    let mut v: Vec<_> = kayitlar()
        .hostlar
        .into_iter()
        .map(|k| KayitliCihaz {
            acik: ACIK_KIMLIKLER.lock().unwrap().contains(&k.izleyici_kimlik),
            kimlik: k.izleyici_kimlik,
            ad: k.etiket.unwrap_or_else(|| k.ad.clone()),
            asil_ad: k.ad,
            son_gorulme: k.son_gorulme,
        })
        .collect();
    v.sort_by_key(|k| std::cmp::Reverse(k.son_gorulme));
    v
}

/// Host: güvenilen cihazı kaldır; o cihaz bir dahaki sefere kod gerektiğini öğrenir.
#[flutter_rust_bridge::frb(sync)]
pub fn guvenilen_kaldir(kimlik: String) {
    if let Some(v) = veri() {
        let _ = guvenilen::guncelle(&guvenilen::kayit_yolu(&v), |k| {
            guvenilen::hosttan_kaldir(k, &kimlik)
        });
    }
}

/// Host olayı. `tur`: hazir | istek | baglandi | koptu | hata | dosya | uyari | yeniden.
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
    pub pano: bool,
    /// baglandi: dosya alma izni.
    pub dosya: bool,
    /// dosya: alınan dosyanın tam yolu.
    pub yol: String,
    pub oyun_kolu: bool,
    /// istek: kayıtlı cihaz (kod olmadan) bağlanmak istiyor.
    pub kayitli: bool,
}

/// İzleyici olayı. `tur`: bekliyor | kabul | kare | istatistik (metin = yol) | yeniden | kaydedildi |
/// koptu | hata | pano | dosya.
#[derive(Debug, Clone, Default)]
pub struct IzleyiciOlayi {
    pub tur: String,
    pub rtt_ms: u32,
    pub fps: u32,
    pub gecikme_ms: u32,
    pub ad: String,
    pub metin: String,
    pub kontrol: bool,
    pub pano: bool,
    pub genislik: u32,
    pub yukseklik: u32,
    pub rgba: Vec<u8>,
    /// kabul: dosya gönderme izni.
    pub dosya: bool,
    pub oyun_kolu: bool,
    /// dosya: gönderilen/toplam bayt; `bitti` ise `metin` boşsa başarılı, doluysa hata.
    pub gonderilen: u64,
    pub toplam: u64,
    pub bitti: bool,
    pub slot: u8,
    pub buyuk: u8,
    pub kucuk: u8,
}

/// Girdi. Terminal: terminal_liste | terminal_ac (metin=id, dx=en, dy=boy) | terminal_kapat.
#[derive(Debug, Clone, Default)]
pub struct GirdiOlayi {
    pub tur: String,
    pub x: f64,
    pub y: f64,
    /// fare: sol | sag | orta; tus: tuş adı.
    pub ad: String,
    pub hid: u32,
    pub basili: bool,
    pub dx: i32,
    pub dy: i32,
    pub metin: String,
}

fn host_dto(o: HostOlay) -> HostOlayi {
    match o {
        HostOlay::Hazir {
            kod,
            parola,
            adresler,
            erisim,
        } => HostOlayi {
            tur: "hazir".into(),
            kod,
            parola,
            adresler,
            erisim,
            ..Default::default()
        },
        HostOlay::Istek { ad, kayitli } => HostOlayi {
            tur: "istek".into(),
            ad,
            kayitli,
            ..Default::default()
        },
        HostOlay::Baglandi { ad, izinler } => HostOlayi {
            tur: "baglandi".into(),
            ad,
            kontrol: izinler.kontrol,
            pano: izinler.pano,
            dosya: izinler.dosya,
            oyun_kolu: izinler.oyun_kolu,
            ..Default::default()
        },
        HostOlay::Koptu { sebep } => HostOlayi {
            tur: "koptu".into(),
            metin: sebep,
            ..Default::default()
        },
        HostOlay::Hata(m) => HostOlayi {
            tur: "hata".into(),
            metin: m,
            ..Default::default()
        },
        HostOlay::DosyaAlindi { ad, yol } => HostOlayi {
            tur: "dosya".into(),
            ad,
            yol,
            ..Default::default()
        },
        HostOlay::Uyari(m) => HostOlayi {
            tur: "uyari".into(),
            metin: m,
            ..Default::default()
        },
        // Yol değişimi (doğrudan/relay): arayüz şimdilik göstermiyor.
        HostOlay::Yol { yol, adres } => HostOlayi {
            tur: "yol".into(),
            metin: yol,
            yol: adres,
            ..Default::default()
        },
        HostOlay::YenidenBekleniyor => HostOlayi {
            tur: "yeniden".into(),
            metin: "Bağlantı koptu; karşı taraf yeniden bağlanıyor…".into(),
            ..Default::default()
        },
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

/// Uygulama açıkken arka planda çalışan host'u başlatır (kayıtlı cihazlar onayla bağlanabilir);
/// kodla bağlantı `host_kod_ac(true)` ile ("Bağlantı ver" ekranı) açılır. Önceki host durdurulur.
pub fn host_baslat(ad: String, parola: String, upnp: bool, olaylar: StreamSink<HostOlayi>) {
    host_durdur();
    let hata = |olaylar: &StreamSink<HostOlayi>, m: String| {
        let _ = olaylar.add(HostOlayi {
            tur: "hata".into(),
            metin: m,
            ..Default::default()
        });
    };
    #[cfg(target_os = "android")]
    {
        let _ = (ad, parola, upnp);
        hata(
            &olaylar,
            "Bu cihazdan bağlantı verme henüz desteklenmiyor; yalnız bağlanabilirsin.".into(),
        );
        return;
    }
    #[cfg(not(target_os = "android"))]
    let fab: Arc<dyn afudesk_core::platform::Fabrika> =
        Arc::new(afudesk_core::platform::masaustu::Gercek);
    #[cfg(not(target_os = "android"))]
    {
        let ayar = HostAyar {
            ad,
            upnp,
            parola,
            yalniz_yerel: false,
            dosya_klasoru: None,
            veri_klasoru: veri(),
            kod_acik: false,
        };
        let mut h = match rt().block_on(host::baslat(ayar, fab)) {
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

pub fn host_kabul(kontrol: bool, pano: bool, dosya: bool, oyun_kolu: bool) {
    host_komut(HostKomut::Kabul(Izinler {
        kontrol,
        pano,
        dosya,
        oyun_kolu,
    }));
}

pub fn host_red() {
    host_komut(HostKomut::Red);
}

pub fn host_kes() {
    host_komut(HostKomut::Kes);
}

/// "Bağlantı ver" ekranı açılınca true, kapanınca false: kod yalnız ekran açıkken geçerli.
#[flutter_rust_bridge::frb(sync)]
pub fn host_kod_ac(acik: bool) {
    if let Some((h, _)) = HOST.lock().unwrap().as_ref() {
        h.kod_ac(acik);
    }
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
pub fn izleyici_baglan(
    kod: String,
    parola: String,
    ad: String,
    olaylar: StreamSink<IzleyiciOlayi>,
) {
    izleyici_kapat();
    IZLEYICI_KOL_IZNI.store(false, Ordering::SeqCst);
    IZLEYICI_KOL_SIRASI.store(0, Ordering::SeqCst);
    // Masaüstünde izleyici panosu arboard; Android'de yok (gelen metni Dart panoya yazar).
    #[cfg(not(target_os = "android"))]
    let pano = afudesk_core::pano::ArboardPano::new()
        .ok()
        .map(|p| Box::new(p) as Box<dyn afudesk_core::pano::Pano>);
    #[cfg(target_os = "android")]
    let pano: Option<Box<dyn afudesk_core::pano::Pano>> = None;
    let v = veri();
    let sonuc = rt().block_on(izleyici::baglan_ayarli(
        &kod,
        &parola,
        &ad,
        pano,
        v.as_deref(),
    ));
    izleyici_pompala(sonuc, olaylar);
}

/// Kayıtlı bilgisayara kod ve parola olmadan bağlan (karşı taraf yine onay verir).
pub fn izleyici_kayitli_baglan(kimlik: String, ad: String, olaylar: StreamSink<IzleyiciOlayi>) {
    izleyici_kapat();
    IZLEYICI_KOL_IZNI.store(false, Ordering::SeqCst);
    IZLEYICI_KOL_SIRASI.store(0, Ordering::SeqCst);
    let Some(v) = veri() else {
        let _ = olaylar.add(IzleyiciOlayi {
            tur: "hata".into(),
            metin: "Kayıtlı cihazlar okunamadı.".into(),
            ..Default::default()
        });
        return;
    };
    #[cfg(not(target_os = "android"))]
    let pano = afudesk_core::pano::ArboardPano::new()
        .ok()
        .map(|p| Box::new(p) as Box<dyn afudesk_core::pano::Pano>);
    #[cfg(target_os = "android")]
    let pano: Option<Box<dyn afudesk_core::pano::Pano>> = None;
    let sonuc = rt().block_on(izleyici::kayitli_baglan(&kimlik, &v, &ad, pano));
    izleyici_pompala(sonuc, olaylar);
}

fn izleyici_pompala(sonuc: anyhow::Result<izleyici::Izleyici>, olaylar: StreamSink<IzleyiciOlayi>) {
    let mut iz = match sonuc {
        Ok(i) => i,
        Err(e) => {
            let _ = olaylar.add(IzleyiciOlayi {
                tur: "hata".into(),
                metin: e.to_string(),
                ..Default::default()
            });
            return;
        }
    };
    iz.parcali_ayarla(true);
    let mut alici = std::mem::replace(&mut iz.olaylar, tokio::sync::mpsc::channel(1).1);
    let iz = Arc::new(iz);
    *IZLEYICI.lock().unwrap() = Some(iz);
    KARE_SERBEST.store(true, Ordering::SeqCst);
    let nesil = IZLEYICI_NESLI.load(Ordering::SeqCst);
    rt().spawn(async move {
        let mut bekleyen = BekleyenParcalar::default();
        loop {
            let o = tokio::select! {
                o = alici.recv() => o,
                _ = KARE_UYANDIR.notified() => {
                    if nesil != IZLEYICI_NESLI.load(Ordering::SeqCst) { break; }
                    if KARE_SERBEST.load(Ordering::SeqCst) {
                        if let Some(dto) = bekleyen.al() {
                            KARE_SERBEST.store(false, Ordering::SeqCst);
                            if olaylar.add(dto).is_err() { break; }
                        }
                    }
                    continue;
                }
            };
            let Some(o) = o else { break; };
            if nesil != IZLEYICI_NESLI.load(Ordering::SeqCst) { break; }
            let dto = match o {
                IzleyiciOlay::OnayBekleniyor { karsi_ad } => IzleyiciOlayi {
                    tur: "bekliyor".into(),
                    ad: karsi_ad,
                    ..Default::default()
                },
                IzleyiciOlay::Kabul {
                    izinler,
                    genislik,
                    yukseklik,
                } => {
                    IZLEYICI_KOL_IZNI.store(izinler.oyun_kolu, Ordering::SeqCst);
                    IzleyiciOlayi {
                        tur: "kabul".into(),
                        kontrol: izinler.kontrol,
                        pano: izinler.pano,
                        dosya: izinler.dosya,
                        oyun_kolu: izinler.oyun_kolu,
                        genislik,
                        yukseklik,
                        ..Default::default()
                    }
                }
                IzleyiciOlay::Kare { genislik, yukseklik, rgba } => {
                    // Normalize the initial legacy frame to the same tile grid.
                    // A delayed full image must never overwrite a newer delta.
                    bekleyen.ekle_kare(genislik, yukseklik, rgba);
                    if !KARE_SERBEST.swap(false, Ordering::SeqCst) { continue; }
                    let Some(dto) = bekleyen.al() else { KARE_SERBEST.store(true, Ordering::SeqCst); continue; };
                    dto
                }
                IzleyiciOlay::Parcalar { genislik, yukseklik, tam, dosemeler, girdi_ms, host_girdi_ms } => {
                    bekleyen.ekle(genislik, yukseklik, tam, dosemeler, girdi_ms, host_girdi_ms);
                    if !KARE_SERBEST.swap(false, Ordering::SeqCst) { continue; }
                    let Some(dto) = bekleyen.al() else { KARE_SERBEST.store(true, Ordering::SeqCst); continue; };
                    dto
                }
                IzleyiciOlay::Imlec(c) => IzleyiciOlayi {
                    tur: "imlec".into(), genislik: c.gen, yukseklik: c.yuk, rgba: c.rgba,
                    metin: serde_json::json!({"x":c.x,"y":c.y,"gorunur":c.gorunur,"sicak_x":c.sicak_x,"sicak_y":c.sicak_y}).to_string(),
                    ..Default::default()
                },
                // metin: kullanılan yol ("doğrudan" | "relay").
                IzleyiciOlay::Istatistik {
                    rtt_ms,
                    fps,
                    gecikme_ms,
                    yol,
                } => IzleyiciOlayi {
                    tur: "istatistik".into(),
                    rtt_ms,
                    fps,
                    gecikme_ms,
                    metin: yol,
                    ..Default::default()
                },
                IzleyiciOlay::YenidenBaglaniyor { deneme } => IzleyiciOlayi {
                    tur: "yeniden".into(),
                    metin: format!("Bağlantı koptu, yeniden bağlanılıyor… ({deneme})"),
                    ..Default::default()
                },
                IzleyiciOlay::Kaydedildi { ad } => IzleyiciOlayi {
                    tur: "kaydedildi".into(),
                    ad,
                    ..Default::default()
                },
                IzleyiciOlay::Koptu { sebep } => IzleyiciOlayi {
                    tur: "koptu".into(),
                    metin: sebep,
                    ..Default::default()
                },
                IzleyiciOlay::Pano(metin) => IzleyiciOlayi {
                    tur: "pano".into(),
                    metin,
                    ..Default::default()
                },
                IzleyiciOlay::TerminalListesi(pencereler) => IzleyiciOlayi {
                    tur: "terminal_liste".into(),
                    metin: serde_json::to_string(&pencereler).unwrap_or_else(|_| "[]".into()),
                    ..Default::default()
                },
                IzleyiciOlay::TerminalDurum { acik, hata } => IzleyiciOlayi {
                    tur: "terminal_durum".into(),
                    metin: hata.unwrap_or_else(|| if acik { "acik" } else { "kapali" }.into()),
                    ..Default::default()
                },
                IzleyiciOlay::Dosya {
                    ad,
                    gonderilen,
                    toplam,
                    bitti,
                    hata,
                } => IzleyiciOlayi {
                    tur: "dosya".into(),
                    ad,
                    gonderilen,
                    toplam,
                    bitti,
                    metin: hata,
                    ..Default::default()
                },
                IzleyiciOlay::KolAlgilandi => IzleyiciOlayi {
                    tur: "kol".into(),
                    ..Default::default()
                },
                IzleyiciOlay::Titresim { slot, buyuk, kucuk } => IzleyiciOlayi {
                    tur: "titresim".into(),
                    slot,
                    buyuk,
                    kucuk,
                    ..Default::default()
                },
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
    KARE_UYANDIR.notify_one();
}

pub fn izleyici_girdi(g: GirdiOlayi) {
    let Some(iz) = IZLEYICI.lock().unwrap().clone() else {
        return;
    };
    if let Some(komut) = terminal_komutu(&g) {
        rt().spawn(async move {
            let _ = iz.terminal_komut(komut).await;
        });
        return;
    }
    let girdi = match g.tur.as_str() {
        "konum" => Girdi::FareKonum {
            x: g.x as f32,
            y: g.y as f32,
        },
        "fare" => Girdi::FareTus {
            tus: match g.ad.as_str() {
                "sag" => FareTusu::Sag,
                "orta" => FareTusu::Orta,
                _ => FareTusu::Sol,
            },
            basili: g.basili,
        },
        "kaydir" => Girdi::Kaydir { dx: g.dx, dy: g.dy },
        "tus" => Girdi::Tus {
            hid: g.hid,
            ad: g.ad,
            basili: g.basili,
        },
        "metin" => Girdi::Metin(g.metin),
        _ => return,
    };
    rt().spawn(async move { iz.gonder(girdi).await });
}

/// Mevcut DTO kullanılır; terminal komutları Girdi olarak karşıya gönderilmez.
fn terminal_komutu(g: &GirdiOlayi) -> Option<Kontrol> {
    match g.tur.as_str() {
        "terminal_liste" => Some(Kontrol::TerminalListesi),
        "terminal_ac" => {
            let id = g.metin.parse::<u64>().ok().filter(|id| *id != 0)?;
            let en = u32::try_from(g.dx).ok().filter(|v| *v > 0)?;
            let boy = u32::try_from(g.dy).ok().filter(|v| *v > 0)?;
            Some(Kontrol::TerminalModu {
                ac: Some(id),
                en,
                boy,
            })
        }
        "terminal_kapat" => Some(Kontrol::TerminalModu {
            ac: None,
            en: 0,
            boy: 0,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod terminal_testleri {
    use super::*;

    #[test]
    fn terminal_kimligi_tam_hassasiyetle_aktarilir() {
        let g = GirdiOlayi {
            tur: "terminal_ac".into(),
            metin: u64::MAX.to_string(),
            dx: 390,
            dy: 844,
            ..Default::default()
        };
        assert_eq!(
            terminal_komutu(&g),
            Some(Kontrol::TerminalModu {
                ac: Some(u64::MAX),
                en: 390,
                boy: 844,
            })
        );
    }

    #[test]
    fn gecersiz_terminal_istekleri_gonderilmez() {
        for (id, en, boy) in [
            ("0", 390, 844),
            ("abc", 390, 844),
            ("42", -1, 844),
            ("42", 390, 0),
        ] {
            assert!(terminal_komutu(&GirdiOlayi {
                tur: "terminal_ac".into(),
                metin: id.into(),
                dx: en,
                dy: boy,
                ..Default::default()
            })
            .is_none());
        }
    }

    #[test]
    fn liste_ve_kapat_girdi_degildir() {
        assert_eq!(
            terminal_komutu(&GirdiOlayi {
                tur: "terminal_liste".into(),
                ..Default::default()
            }),
            Some(Kontrol::TerminalListesi)
        );
        assert_eq!(
            terminal_komutu(&GirdiOlayi {
                tur: "terminal_kapat".into(),
                ..Default::default()
            }),
            Some(Kontrol::TerminalModu {
                ac: None,
                en: 0,
                boy: 0
            })
        );
        assert!(terminal_komutu(&GirdiOlayi {
            tur: "metin".into(),
            metin: "terminal_ac".into(),
            ..Default::default()
        })
        .is_none());
    }
}

/// Dart'tan gelen oyun kolu durumunu mevcut datagram yoluna verir; sıra numarasını Rust atar.
pub fn izleyici_kol(
    slot: u8,
    dugmeler: u16,
    sol_x: i16,
    sol_y: i16,
    sag_x: i16,
    sag_y: i16,
    sol_tetik: u8,
    sag_tetik: u8,
) {
    if !IZLEYICI_KOL_IZNI.load(Ordering::SeqCst) {
        return;
    }
    let Some(iz) = IZLEYICI.lock().unwrap().clone() else {
        return;
    };
    let sira = IZLEYICI_KOL_SIRASI.fetch_add(1, Ordering::SeqCst);
    rt().spawn(async move {
        iz.gonder(Girdi::Kol(afudesk_core::protokol::KolDurumu {
            slot,
            sira,
            dugmeler,
            sol_x,
            sol_y,
            sag_x,
            sag_y,
            sol_tetik,
            sag_tetik,
        }))
        .await;
    });
}

/// Dosyayı karşı tarafa gönderir; ilerleme/sonuç izleyici akışına `tur = "dosya"` olarak gelir.
pub fn izleyici_dosya_gonder(yol: String) {
    let Some(iz) = IZLEYICI.lock().unwrap().clone() else {
        return;
    };
    let _g = rt().enter();
    // Görev arka planda sürer; sonuç olay olarak gelir.
    drop(iz.dosya_gonder(yol));
}

pub fn izleyici_kapat() {
    IZLEYICI_NESLI.fetch_add(1, Ordering::SeqCst);
    KARE_UYANDIR.notify_waiters();
    if let Some(iz) = IZLEYICI.lock().unwrap().take() {
        iz.kapat();
    }
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

#[flutter_rust_bridge::frb(sync)]
pub fn kayitli_yeniden_adla(kimlik: String, ad: String) {
    if let Some(v) = veri() {
        let _ = guvenilen::guncelle(&guvenilen::kayit_yolu(&v), |k| {
            guvenilen::etiketle(k, &kimlik, Some(ad), false)
        });
    }
}
#[flutter_rust_bridge::frb(sync)]
pub fn guvenilen_yeniden_adla(kimlik: String, ad: String) {
    if let Some(v) = veri() {
        let _ = guvenilen::guncelle(&guvenilen::kayit_yolu(&v), |k| {
            guvenilen::etiketle(k, &kimlik, Some(ad), true)
        });
    }
}
pub async fn cevrimici_kimlikler() -> Vec<String> {
    let kimlik = HOST.lock().unwrap().as_ref().map(|(h, _)| h.kimlik());
    let kimlikler = afudesk_core::ag::cevrimici_kimlikler(kimlik).await;
    *ACIK_KIMLIKLER.lock().unwrap() = kimlikler.clone();
    kimlikler
}

#[cfg(test)]
mod parca_testleri {
    use super::*;
    #[test]
    fn legacy_base_and_newer_delta_share_one_tile_queue() {
        let mut p = BekleyenParcalar::default();
        p.ekle_kare(65, 1, [1, 0, 0, 255].repeat(65));
        p.ekle(65, 1, false, vec![afudesk_core::goruntu::DosemeRgba {
            x: 0, y: 0, gen: 64, yuk: 1, rgba: [3, 0, 0, 255].repeat(64),
        }], 11, 21);
        let dto = p.al().unwrap();
        assert_eq!(dto.tur, "parcalar");
        assert_eq!(dto.rgba[16], 3);
        assert_eq!(dto.rgba[16 + 64 * 4 + 16], 1);
        assert!(serde_json::from_str::<serde_json::Value>(&dto.metin).unwrap()["tam"].as_bool().unwrap());
        assert!(p.al().is_none());
    }
    #[test]
    fn newer_full_image_replaces_pending_delta() {
        let mut p = BekleyenParcalar::default();
        p.ekle(1, 1, false, vec![tile(0, 3)], 11, 21);
        p.ekle_kare(1, 1, vec![9, 0, 0, 255]);
        assert_eq!(p.al().unwrap().rgba[16], 9);
        assert!(p.al().is_none());
    }
    fn tile(x: u32, color: u8) -> afudesk_core::goruntu::DosemeRgba {
        afudesk_core::goruntu::DosemeRgba {
            x,
            y: 0,
            gen: 1,
            yuk: 1,
            rgba: vec![color, 0, 0, 255],
        }
    }
    #[test]
    fn pending_deltas_keep_untouched_tiles_and_latest_pixel() {
        let mut p = BekleyenParcalar::default();
        p.ekle(2, 1, true, vec![tile(0, 1), tile(1, 2)], 10, 20);
        p.ekle(2, 1, false, vec![tile(0, 3)], 11, 21);
        let dto = p.al().unwrap();
        assert_eq!(dto.rgba.len(), 40);
        assert_eq!(dto.rgba[16], 3);
        assert_eq!(dto.rgba[36], 2);
        let meta: serde_json::Value = serde_json::from_str(&dto.metin).unwrap();
        assert_eq!(meta["tam"], true);
        assert_eq!(meta["girdi_ms"], 11);
        assert!(p.al().is_none());
    }
    #[test]
    fn full_reset_discards_previous_surface_tiles() {
        let mut p = BekleyenParcalar::default();
        p.ekle(2, 1, true, vec![tile(0, 1), tile(1, 2)], 10, 20);
        p.ekle(1, 1, true, vec![tile(0, 3)], 11, 21);
        let dto = p.al().unwrap();
        assert_eq!((dto.genislik, dto.yukseklik, dto.rgba.len()), (1, 1, 20));
        assert_eq!(dto.rgba[16], 3);
    }
}
