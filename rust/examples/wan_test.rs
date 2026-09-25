//! WAN test düzeneği (TEST-1 / TEST-2): gerçek ekran yakalama + gerçek fare/klavye,
//! iki ayrı süreç. Sonuçlar klasöre JSON + PNG olarak yazılır; PASS/FAIL kararını
//! `scripts/wan_test/calistir.py` verir.
//!
//!   wan_test host <klasor>          kod.txt yazar, isteği kendisi onaylar, gelen her
//!                                   girdiyi uygular ve işletim sisteminden doğrular
//!   wan_test izle <klasor> [sn]     kod.txt ile bağlanır, kareleri/yolu ölçer, fare ve
//!                                   Shift gönderir, ortada bir kez kopup geri döner
//!
//! Güvenlik: tıklama gönderilmez; yalnız imleç hareketi ve Shift bas/bırak.
use afudesk_core::{
    host::{self, HostAyar, HostKomut, HostOlay},
    izleyici::{self, IzleyiciOlay},
    pano::Pano,
    platform::{masaustu::Gercek, Enjektor, Fabrika, Yakalayici},
    protokol::{Girdi, Izinler},
    sanal_kol::KolSurucusu,
};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

fn simdi_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn yaz_json(yol: &Path, v: &Value) -> Result<()> {
    std::fs::write(yol, serde_json::to_vec_pretty(v)?)?;
    Ok(())
}

#[cfg(windows)]
mod os {
    use windows_sys::Win32::{
        Foundation::POINT,
        UI::{
            Input::KeyboardAndMouse::GetAsyncKeyState,
            WindowsAndMessaging::{GetCursorPos, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN},
        },
    };
    pub fn imlec() -> (i32, i32) {
        let mut p = POINT { x: -1, y: -1 };
        unsafe { GetCursorPos(&mut p) };
        (p.x, p.y)
    }
    pub fn ekran() -> (i32, i32) {
        unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) }
    }
    pub fn shift_basili() -> bool {
        (unsafe { GetAsyncKeyState(0x10) } as u16 & 0x8000) != 0
    }
}

// ---------------------------------------------------------------- host

struct KayitliEnjektor {
    ic: Box<dyn Enjektor>,
    kayit: Arc<Mutex<Vec<Value>>>,
}

impl Enjektor for KayitliEnjektor {
    fn uygula(&mut self, g: &Girdi) -> Result<()> {
        let r = self.ic.uygula(g);
        std::thread::sleep(Duration::from_millis(40));
        let v = match g {
            Girdi::FareKonum { x, y } => {
                let (w, h) = os::ekran();
                let beklenen = (
                    (x.clamp(0.0, 1.0) * (w - 1) as f32).round() as i32,
                    (y.clamp(0.0, 1.0) * (h - 1) as f32).round() as i32,
                );
                let gercek = os::imlec();
                json!({"t": simdi_ms(), "tur": "fare", "x": x, "y": y,
                       "beklenen": [beklenen.0, beklenen.1], "gercek": [gercek.0, gercek.1],
                       "uygulandi": r.is_ok()})
            }
            Girdi::Tus { ad, basili } => {
                json!({"t": simdi_ms(), "tur": "tus", "ad": ad, "basili": basili,
                       "os_shift_basili": os::shift_basili(), "uygulandi": r.is_ok()})
            }
            _ => json!({"t": simdi_ms(), "tur": "diger", "uygulandi": r.is_ok()}),
        };
        self.kayit.lock().unwrap().push(v);
        r
    }
}

struct Izleyen {
    ic: Gercek,
    kayit: Arc<Mutex<Vec<Value>>>,
}

impl Fabrika for Izleyen {
    fn yakalayici(&self) -> Result<Box<dyn Yakalayici>> {
        self.ic.yakalayici()
    }
    fn enjektor(&self) -> Result<Box<dyn Enjektor>> {
        Ok(Box::new(KayitliEnjektor {
            ic: self.ic.enjektor()?,
            kayit: self.kayit.clone(),
        }))
    }
    fn pano(&self) -> Result<Box<dyn Pano>> {
        self.ic.pano()
    }
    fn kol_surucusu(&self) -> Result<Box<dyn KolSurucusu>> {
        self.ic.kol_surucusu()
    }
}

async fn host_kipi(klasor: PathBuf) -> Result<()> {
    let kayit = Arc::new(Mutex::new(Vec::new()));
    let mut h = host::baslat(
        HostAyar {
            ad: "WAN-TEST-HOST".into(),
            upnp: true,
            parola: String::new(),
            yalniz_yerel: false,
            dosya_klasoru: None,
            veri_klasoru: None,
            kod_acik: true,
        },
        Arc::new(Izleyen {
            ic: Gercek,
            kayit: kayit.clone(),
        }),
    )
    .await?;
    let komut = h.komut_gonderici();
    let mut olaylar: Vec<Value> = Vec::new();
    let dur = klasor.join("dur");
    let bitis = Instant::now() + Duration::from_secs(15 * 60);
    let mut kontrol = tokio::time::interval(Duration::from_millis(300));
    while Instant::now() < bitis && !dur.exists() {
        tokio::select! {
            o = h.olaylar.recv() => {
                let Some(o) = o else { break };
                let v = match &o {
                    HostOlay::Hazir { kod, parola, adresler, erisim } => {
                        let gecici = klasor.join("kod.tmp");
                        std::fs::write(&gecici, format!("{kod}\n{parola}\n"))?;
                        std::fs::rename(&gecici, klasor.join("kod.txt"))?;
                        json!({"tur": "hazir", "adresler": adresler, "erisim": erisim})
                    }
                    HostOlay::Istek { ad, .. } => {
                        let _ = komut.send(HostKomut::Kabul(Izinler {
                            kontrol: true, pano: false, dosya: false, oyun_kolu: false,
                        })).await;
                        json!({"tur": "istek", "ad": ad})
                    }
                    HostOlay::Baglandi { ad, .. } => json!({"tur": "baglandi", "ad": ad}),
                    HostOlay::Yol { yol, adres } => json!({"tur": "yol", "yol": yol, "adres": adres}),
                    HostOlay::YenidenBekleniyor => json!({"tur": "yeniden_bekleniyor"}),
                    HostOlay::Koptu { sebep } => json!({"tur": "koptu", "sebep": sebep}),
                    HostOlay::Hata(m) => json!({"tur": "hata", "metin": m}),
                    HostOlay::Uyari(m) => json!({"tur": "uyari", "metin": m}),
                    HostOlay::DosyaAlindi { ad, .. } => json!({"tur": "dosya", "ad": ad}),
                };
                let mut v = v;
                v["t"] = json!(simdi_ms());
                println!("HOST {v}");
                olaylar.push(v);
                yaz_json(&klasor.join("host_rapor.json"),
                         &json!({"olaylar": olaylar, "girdiler": *kayit.lock().unwrap()}))?;
            }
            _ = kontrol.tick() => {}
        }
    }
    h.durdur();
    tokio::time::sleep(Duration::from_millis(500)).await;
    yaz_json(
        &klasor.join("host_rapor.json"),
        &json!({"olaylar": olaylar, "girdiler": *kayit.lock().unwrap()}),
    )?;
    Ok(())
}

// ---------------------------------------------------------------- izleyici

fn parlaklik_sapmasi(rgba: &[u8]) -> f64 {
    let n = rgba.len() / 4;
    if n == 0 {
        return 0.0;
    }
    let l: Vec<f64> = rgba
        .chunks_exact(4)
        .step_by(7)
        .map(|p| 0.299 * p[0] as f64 + 0.587 * p[1] as f64 + 0.114 * p[2] as f64)
        .collect();
    let ort = l.iter().sum::<f64>() / l.len() as f64;
    (l.iter().map(|v| (v - ort).powi(2)).sum::<f64>() / l.len() as f64).sqrt()
}

fn png_kaydet(yol: &Path, g: u32, y: u32, rgba: &[u8]) {
    if let Some(img) = image::RgbaImage::from_raw(g, y, rgba.to_vec()) {
        let _ = img.save(yol);
    }
}

async fn izle_kipi(klasor: PathBuf, sure: Duration) -> Result<()> {
    let kod_yolu = klasor.join("kod.txt");
    let bekle_bitis = Instant::now() + Duration::from_secs(90);
    while !kod_yolu.exists() {
        anyhow::ensure!(Instant::now() < bekle_bitis, "kod.txt gelmedi");
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let metin = std::fs::read_to_string(&kod_yolu)?;
    let mut satir = metin.lines();
    let (kod, parola) = (
        satir.next().context("kod yok")?.to_owned(),
        satir.next().context("parola yok")?.to_owned(),
    );
    let t0 = Instant::now();
    let mut rapor = json!({
        "ortam": {
            "AFUDESK_SADECE_RELAY": std::env::var("AFUDESK_SADECE_RELAY").unwrap_or_default(),
            "AFUDESK_BAGLA_IP": std::env::var("AFUDESK_BAGLA_IP").unwrap_or_default(),
            "AFUDESK_PROXY": std::env::var("AFUDESK_PROXY").unwrap_or_default(),
            "AFUDESK_TEST_GENEL_YOL": std::env::var("AFUDESK_TEST_GENEL_YOL").unwrap_or_default(),
        }
    });
    let mut i = match izleyici::baglan(&kod, &parola, "WAN-TEST-IZLEYICI").await {
        Ok(i) => i,
        Err(e) => {
            rapor["hata"] = json!(format!("{e:#}"));
            yaz_json(&klasor.join("izle_rapor.json"), &rapor)?;
            return Err(e);
        }
    };
    rapor["baglanma_ms"] = json!(t0.elapsed().as_millis());
    let mut olaylar: Vec<Value> = Vec::new();
    let mut istatistik: Vec<Value> = Vec::new();
    let mut kabul_zamani: Option<Instant> = None;
    let mut kabul_sayisi = 0u32;
    let (mut kare, mut kare_sonra) = (0u64, 0u64);
    let mut ilk_kare_sapma = None;
    let mut son_kare: Option<(u32, u32, Vec<u8>)> = None;
    let mut yeniden_basladi: Option<Instant> = None;
    let mut yeniden_ms: Option<u128> = None;
    let mut adim = 0u8;
    let mut saat = tokio::time::interval(Duration::from_millis(100));
    let son = Instant::now() + sure + Duration::from_secs(40);
    let mut koptu = None;
    loop {
        if Instant::now() > son {
            break;
        }
        if let Some(tk) = kabul_zamani {
            let gecen = tk.elapsed();
            // Plan: fare, fare, Shift, kopuş+dönüş, dönüşten sonra fare, bitiş.
            if adim == 0 && gecen > Duration::from_secs(4) {
                i.gonder(Girdi::FareKonum { x: 0.30, y: 0.40 }).await;
                adim = 1;
            } else if adim == 1 && gecen > Duration::from_secs(5) {
                i.gonder(Girdi::FareKonum { x: 0.62, y: 0.55 }).await;
                adim = 2;
            } else if adim == 2 && gecen > Duration::from_secs(6) {
                i.gonder(Girdi::Tus { ad: "Shift".into(), basili: true }).await;
                adim = 3;
            } else if adim == 3 && gecen > Duration::from_millis(6_400) {
                i.gonder(Girdi::Tus { ad: "Shift".into(), basili: false }).await;
                adim = 4;
            } else if adim == 4 && gecen > Duration::from_secs(10) {
                olaylar.push(json!({"t": simdi_ms(), "tur": "yeniden_baglan_istendi"}));
                yeniden_basladi = Some(Instant::now());
                i.yeniden_baglan();
                adim = 5;
            } else if adim == 6 && kare_sonra > 10 {
                i.gonder(Girdi::FareKonum { x: 0.45, y: 0.50 }).await;
                adim = 7;
            } else if adim == 7 && gecen > sure {
                break;
            }
        }
        tokio::select! {
            o = i.olaylar.recv() => {
                let Some(o) = o else { break };
                match o {
                    IzleyiciOlay::Kare { genislik, yukseklik, rgba } => {
                        kare += 1;
                        if adim >= 6 { kare_sonra += 1; }
                        if kare == 15 {
                            ilk_kare_sapma = Some(parlaklik_sapmasi(&rgba));
                            png_kaydet(&klasor.join("ilk_kare.png"), genislik, yukseklik, &rgba);
                        }
                        son_kare = Some((genislik, yukseklik, rgba));
                    }
                    IzleyiciOlay::Istatistik { rtt_ms, fps, gecikme_ms, yol } => {
                        let v = json!({"t": simdi_ms(), "rtt_ms": rtt_ms, "fps": fps,
                                       "gecikme_ms": gecikme_ms, "yol": yol, "adres": i.yol_adresi(),
                                       "donus_sonrasi": adim >= 6});
                        println!("IZLE {v}");
                        istatistik.push(v);
                    }
                    IzleyiciOlay::Kabul { genislik, yukseklik, .. } => {
                        kabul_sayisi += 1;
                        if kabul_zamani.is_none() {
                            kabul_zamani = Some(Instant::now());
                        } else if let Some(b) = yeniden_basladi.take() {
                            yeniden_ms = Some(b.elapsed().as_millis());
                            adim = 6;
                        }
                        olaylar.push(json!({"t": simdi_ms(), "tur": "kabul", "boyut": [genislik, yukseklik]}));
                    }
                    IzleyiciOlay::YenidenBaglaniyor { deneme } => {
                        olaylar.push(json!({"t": simdi_ms(), "tur": "yeniden", "deneme": deneme}));
                    }
                    IzleyiciOlay::Koptu { sebep } => {
                        olaylar.push(json!({"t": simdi_ms(), "tur": "koptu", "sebep": sebep}));
                        koptu = Some(sebep);
                        break;
                    }
                    IzleyiciOlay::OnayBekleniyor { karsi_ad } => {
                        olaylar.push(json!({"t": simdi_ms(), "tur": "onay_bekleniyor", "karsi": karsi_ad}));
                    }
                    _ => {}
                }
            }
            _ = saat.tick() => {}
        }
    }
    if let Some((g, y, rgba)) = &son_kare {
        png_kaydet(&klasor.join("son_kare.png"), *g, *y, rgba);
    }
    let sure_sn = kabul_zamani.map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0);
    rapor["kabul_sayisi"] = json!(kabul_sayisi);
    rapor["kare"] = json!(kare);
    rapor["kare_donus_sonrasi"] = json!(kare_sonra);
    rapor["ort_fps"] = json!(if sure_sn > 0.0 { kare as f64 / sure_sn } else { 0.0 });
    rapor["ilk_kare_parlaklik_sapmasi"] = json!(ilk_kare_sapma);
    rapor["son_kare_boyut"] = json!(son_kare.as_ref().map(|(g, y, _)| [*g, *y]));
    rapor["yeniden_baglanma_ms"] = json!(yeniden_ms);
    rapor["plan_adimi"] = json!(adim);
    rapor["koptu"] = json!(koptu);
    rapor["olaylar"] = json!(olaylar);
    rapor["istatistik"] = json!(istatistik);
    yaz_json(&klasor.join("izle_rapor.json"), &rapor)?;
    i.kapat();
    tokio::time::sleep(Duration::from_millis(500)).await;
    Ok(())
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let kip = a.get(1).map(String::as_str).unwrap_or("");
    let klasor = PathBuf::from(a.get(2).context("kullanım: wan_test host|izle <klasor> [sn]")?);
    std::fs::create_dir_all(&klasor)?;
    match kip {
        "host" => host_kipi(klasor).await,
        "izle" => {
            let sn = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(25);
            izle_kipi(klasor, Duration::from_secs(sn)).await
        }
        _ => anyhow::bail!("kip: host | izle"),
    }
}
