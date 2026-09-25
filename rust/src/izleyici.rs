//! Bağlanan taraf: kodu çözer, bağlanır (doğrudan, NAT delme ya da relay), onayı bekler,
//! kareleri birleştirir. Bağlantı istemeden koparsa aynı cihaz kimliği ve host'un verdiği
//! devam jetonuyla kendiliğinden yeniden bağlanır; kullanıcıdan yeni kod istenmez.
use crate::{
    ag::{self, AlAkisi, Baglanti, GonderAkisi, Kurulum},
    dosya::{self, GonderAyari, Gonderim},
    goruntu::Birlestirici,
    host::{istemsiz_kopus, KOPUS_KODU},
    kod,
    pano::{self, Esitleyici, Pano, PANO_ARALIGI},
    protokol::{self, Girdi, Izinler, Kare, Kontrol, SURUM},
    zaman,
};
use anyhow::Result;
use iroh::{Endpoint, EndpointAddr};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicI64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::sync::mpsc;

/// İlk bağlantı için süre: NAT delme + relay üzerinden el sıkışma dahil.
pub const BAGLANMA_SURESI: Duration = Duration::from_secs(20);
/// Kopan bağlantıyı geri kurmak için toplam süre. Host kopuşu boşta kalma süresiyle
/// (20 sn) fark edip `host::DEVAM_SURESI` (60 sn) bekler; bu ikisini kapsar.
pub const YENIDEN_SURESI: Duration = Duration::from_secs(90);

#[derive(Debug, Clone, PartialEq)]
pub enum IzleyiciOlay {
    /// Bağlandı, karşı tarafın onayı bekleniyor.
    OnayBekleniyor {
        karsi_ad: String,
    },
    Kabul {
        izinler: Izinler,
        genislik: u32,
        yukseklik: u32,
    },
    /// Birleştirilmiş tam ekran (RGBA).
    Kare {
        genislik: u32,
        yukseklik: u32,
        rgba: Vec<u8>,
    },
    /// Saniyede bir: bağlantı gidiş-dönüş süresi, ekrana gelen kare hızı ve kullanılan yol
    /// ("doğrudan" | "relay").
    Istatistik {
        rtt_ms: u32,
        fps: u32,
        gecikme_ms: u32,
        yol: String,
    },
    /// Bağlantı koptu, yeniden deneniyor (`deneme` 1'den başlar).
    YenidenBaglaniyor {
        deneme: u32,
    },
    Koptu {
        sebep: String,
    },
    Pano(String),
    Dosya {
        ad: String,
        gonderilen: u64,
        toplam: u64,
        bitti: bool,
        hata: String,
    },
    KolAlgilandi,
    Titresim {
        slot: u8,
        buyuk: u8,
        kucuk: u8,
    },
}

pub struct Izleyici {
    pub olaylar: mpsc::Receiver<IzleyiciOlay>,
    girdi: mpsc::Sender<Girdi>,
    /// Yeniden bağlanınca değişir.
    baglanti: Arc<Mutex<Baglanti>>,
    kapatildi: Arc<AtomicBool>,
    yeniden_iste: Arc<AtomicBool>,
    /// Zayıf: oturum bitince olay kanalı kapanabilsin.
    olay: mpsc::WeakSender<IzleyiciOlay>,
    ep: Endpoint,
}

impl Izleyici {
    fn simdiki(&self) -> Baglanti {
        self.baglanti.lock().expect("kilit").clone()
    }

    /// Girdi kuyruğu doluysa (yavaş ağ) fare hareketleri düşürülür, diğerleri beklenir.
    pub async fn gonder(&self, g: Girdi) {
        if matches!(g, Girdi::FareKonum { .. }) {
            let _ = self.girdi.try_send(g);
        } else {
            let _ = self.girdi.send(g).await;
        }
    }

    pub fn kapat(&self) {
        self.kapatildi.store(true, Ordering::SeqCst);
        self.simdiki().close(0u32.into(), b"izleyici");
        let ep = self.ep.clone();
        tokio::spawn(async move { ep.close().await });
    }

    /// Bağlantıyı bırakıp aynı oturuma yeniden bağlanır (ör. ağ değişti). Host bunu
    /// kopuş sayar ve onay sormadan geri alır.
    pub fn yeniden_baglan(&self) {
        self.yeniden_iste.store(true, Ordering::SeqCst);
        self.simdiki().close(KOPUS_KODU.into(), b"ag-degisti");
    }

    /// Şu an veri taşıyan yol ve RTT.
    pub fn yol(&self) -> (ag::Yol, Duration) {
        ag::secili_yol(&self.simdiki())
    }

    /// Seçili yolun karşı ucu (IP:port ya da relay adresi).
    pub fn yol_adresi(&self) -> String {
        ag::secili_yol_adresi(&self.simdiki())
    }

    /// Dosyayı karşı tarafa gönderir (arka planda). İlerleme ve sonuç
    /// `IzleyiciOlay::Dosya` olarak gelir; sonuç ayrıca dönen görevden okunabilir.
    /// Aynı dosya yeniden gönderilirse host kaldığı yerden devam ettirir.
    pub fn dosya_gonder(
        &self,
        yol: impl Into<PathBuf>,
    ) -> tokio::task::JoinHandle<Result<Gonderim>> {
        self.dosya_gonder_ayarli(yol.into(), GonderAyari::default())
    }

    pub(crate) fn dosya_gonder_ayarli(
        &self,
        yol: PathBuf,
        ayar: GonderAyari,
    ) -> tokio::task::JoinHandle<Result<Gonderim>> {
        let c = self.simdiki();
        let olay = self.olay.upgrade();
        tokio::spawn(async move {
            let ad = yol
                .file_name()
                .map(|a| a.to_string_lossy().into_owned())
                .unwrap_or_default();
            let mut son = (0, 0);
            let sonuc = dosya::gonder(&c, &yol, ayar, |gonderilen, toplam| {
                son = (gonderilen, toplam);
                // Ara ilerleme düşebilir (arayüz yavaşsa); sonuç asla düşmez.
                if let Some(o) = &olay {
                    let _ = o.try_send(IzleyiciOlay::Dosya {
                        ad: ad.clone(),
                        gonderilen,
                        toplam,
                        bitti: false,
                        hata: String::new(),
                    });
                }
            })
            .await;
            if let Some(o) = &olay {
                let bitis = match &sonuc {
                    Ok(g) => IzleyiciOlay::Dosya {
                        ad: ad.clone(),
                        gonderilen: g.toplam,
                        toplam: g.toplam,
                        bitti: true,
                        hata: String::new(),
                    },
                    Err(e) => IzleyiciOlay::Dosya {
                        ad: ad.clone(),
                        gonderilen: son.0,
                        toplam: son.1,
                        bitti: true,
                        hata: e.to_string(),
                    },
                };
                let _ = o.send(bitis).await;
            }
            sonuc
        })
    }
}

pub async fn baglan(kod_metni: &str, parola: &str, ad: &str) -> Result<Izleyici> {
    baglan_panolu(kod_metni, parola, ad, None).await
}

/// Kodda yalnız yerel döngü adresi varsa (testler) dışarı hiçbir istek çıkarılmaz.
fn kurulum_sec(d: &kod::Davet) -> Kurulum {
    let yalniz_yerel = d.relaylar.is_empty()
        && !d.adresler.is_empty()
        && d.adresler.iter().all(|a| {
            a.parse::<std::net::SocketAddr>()
                .is_ok_and(|s| s.ip().is_loopback())
        });
    if yalniz_yerel {
        Kurulum::YalnizYerel
    } else {
        Kurulum::ortamdan()
    }
}

/// `pano`: izleyicinin kendi panosu. `None` (ör. Android) ise yalnız host → izleyici yönü
/// çalışır; gelen metin `IzleyiciOlay::Pano` ile arayüze bildirilir.
pub async fn baglan_panolu(
    kod_metni: &str,
    parola: &str,
    ad: &str,
    pano: Option<Box<dyn Pano>>,
) -> Result<Izleyici> {
    let d = kod::coz(kod_metni, parola, kod::simdi())?;
    let ep = ag::uc_nokta(kurulum_sec(&d), false).await?;
    let hedef = ag::hedef_adres(&d.parmak_izi, &d.adresler, &d.relaylar)?;
    let c = ag::baglan(&ep, hedef.clone(), BAGLANMA_SURESI).await?;
    let (mut w, r) = c.open_bi().await?;
    protokol::yaz(
        &mut w,
        &Kontrol::Merhaba {
            surum: SURUM,
            bilet: d.bilet.clone(),
            ad: ad.to_owned(),
        },
    )
    .await?;
    let (olay_tx, olaylar) = mpsc::channel(4);
    let (girdi, girdi_rx) = mpsc::channel::<Girdi>(256);
    let _ = olay_tx
        .send(IzleyiciOlay::OnayBekleniyor {
            karsi_ad: d.ad.clone(),
        })
        .await;
    let olay = olay_tx.downgrade();

    let (kol_olay_tx, kol_olay_rx) = mpsc::unbounded_channel();
    let (titresim_tx, titresim_rx) = std::sync::mpsc::channel::<(u8, u8)>();
    #[cfg(all(not(target_os = "android"), not(test)))]
    {
        let tx = girdi.clone();
        std::thread::spawn(move || crate::kol::oku(tx, kol_olay_tx, titresim_rx));
    }
    #[cfg(any(target_os = "android", test))]
    drop(kol_olay_tx);
    #[cfg(any(target_os = "android", test))]
    drop(titresim_rx);

    let baglanti = Arc::new(Mutex::new(c.clone()));
    let kapatildi = Arc::new(AtomicBool::new(false));
    let yeniden_iste = Arc::new(AtomicBool::new(false));
    let gozetmen = Gozetmen {
        ep: ep.clone(),
        hedef,
        ad: ad.to_owned(),
        baglanti: baglanti.clone(),
        kapatildi: kapatildi.clone(),
        yeniden_iste: yeniden_iste.clone(),
        olay: olay_tx,
        girdi: girdi_rx,
        pano: PanoDurumu {
            kaynak: pano,
            es: None,
        },
        saat_farki: Arc::new(AtomicI64::new(0)),
        saat_esitlendi: Arc::new(AtomicBool::new(false)),
        kol_olay: kol_olay_rx,
        titresim: titresim_tx,
    };
    tokio::spawn(gozetmen.calis(c, w, r));
    Ok(Izleyici {
        olaylar,
        girdi,
        baglanti,
        kapatildi,
        yeniden_iste,
        olay,
        ep,
    })
}

struct PanoDurumu {
    kaynak: Option<Box<dyn Pano>>,
    es: Option<Esitleyici>,
}

/// Oturumu yürütür; bağlantı koparsa yeniden kurar.
struct Gozetmen {
    ep: Endpoint,
    hedef: EndpointAddr,
    ad: String,
    baglanti: Arc<Mutex<Baglanti>>,
    kapatildi: Arc<AtomicBool>,
    yeniden_iste: Arc<AtomicBool>,
    olay: mpsc::Sender<IzleyiciOlay>,
    girdi: mpsc::Receiver<Girdi>,
    pano: PanoDurumu,
    saat_farki: Arc<AtomicI64>,
    saat_esitlendi: Arc<AtomicBool>,
    kol_olay: mpsc::UnboundedReceiver<()>,
    titresim: std::sync::mpsc::Sender<(u8, u8)>,
}

/// Yeniden bağlanma denemeleri arasındaki bekleme: 0.5, 1, 2, 4, 5, 5… sn.
pub fn bekleme(deneme: u32) -> Duration {
    let ms = 500u64.saturating_mul(1 << deneme.saturating_sub(1).min(4));
    Duration::from_millis(ms.min(5_000))
}

impl Gozetmen {
    async fn calis(mut self, mut c: Baglanti, mut w: GonderAkisi, mut r: AlAkisi) {
        let mut jeton: Option<String> = None;
        let sebep = loop {
            let sebep = self.oturum(&c, &mut w, r, &mut jeton).await;
            let istendi = self.yeniden_iste.swap(false, Ordering::SeqCst);
            let geri_don = !self.kapatildi.load(Ordering::SeqCst)
                && jeton.is_some()
                && (istendi || istemsiz_kopus(&c));
            if !geri_don {
                c.close(0u32.into(), b"bitti");
                break sebep;
            }
            match self.yeniden_kur(jeton.as_deref().unwrap_or_default()).await {
                Some((c2, w2, r2)) => {
                    *self.baglanti.lock().expect("kilit") = c2.clone();
                    (c, w, r) = (c2, w2, r2);
                }
                None => break "Bağlantı koptu; yeniden bağlanılamadı.".into(),
            }
        };
        let _ = self.olay.send(IzleyiciOlay::Koptu { sebep }).await;
        self.ep.close().await;
    }

    async fn yeniden_kur(&mut self, jeton: &str) -> Option<(Baglanti, GonderAkisi, AlAkisi)> {
        let bitis = tokio::time::Instant::now() + YENIDEN_SURESI;
        let mut deneme = 0u32;
        while tokio::time::Instant::now() < bitis && !self.kapatildi.load(Ordering::SeqCst) {
            deneme += 1;
            let _ = self
                .olay
                .send(IzleyiciOlay::YenidenBaglaniyor { deneme })
                .await;
            let kalan = bitis.saturating_duration_since(tokio::time::Instant::now());
            let sure = BAGLANMA_SURESI.min(kalan).max(Duration::from_secs(1));
            if let Ok(c) = ag::baglan(&self.ep, self.hedef.clone(), sure).await {
                if let Ok((mut w, r)) = c.open_bi().await {
                    let devam = Kontrol::Devam {
                        surum: SURUM,
                        jeton: jeton.to_owned(),
                        ad: self.ad.clone(),
                    };
                    if protokol::yaz(&mut w, &devam).await.is_ok() {
                        return Some((c, w, r));
                    }
                }
            }
            tokio::time::sleep(bekleme(deneme)).await;
        }
        None
    }

    /// Tek bağlantı üzerindeki oturum; bitiş sebebini döndürür.
    async fn oturum(
        &mut self,
        c: &Baglanti,
        w: &mut GonderAkisi,
        mut r: AlAkisi,
        jeton: &mut Option<String>,
    ) -> String {
        let olay = self.olay.clone();
        let ilk =
            tokio::time::timeout(Duration::from_secs(75), protokol::oku::<_, Kontrol>(&mut r))
                .await;
        let izinler = match ilk {
            Ok(Ok(Some(Kontrol::Kabul {
                izinler,
                genislik,
                yukseklik,
            }))) => {
                let _ = olay
                    .send(IzleyiciOlay::Kabul {
                        izinler: izinler.clone(),
                        genislik,
                        yukseklik,
                    })
                    .await;
                izinler
            }
            Ok(Ok(Some(Kontrol::Red(s)))) => {
                // Red'den sonra geri dönülmez.
                *jeton = None;
                return s;
            }
            Err(_) => return "Karşı taraf yanıt vermedi.".into(),
            _ => return "Bağlantı koptu.".into(),
        };
        let (gelen_tx, mut gelen_rx) = mpsc::channel::<Kontrol>(16);
        let okuyucu = tokio::spawn(async move {
            while let Ok(Some(m)) = protokol::oku::<_, Kontrol>(&mut r).await {
                if gelen_tx.send(m).await.is_err() {
                    break;
                }
            }
        });
        let c2 = c.clone();
        let olay2 = olay.clone();
        let fark2 = self.saat_farki.clone();
        let esit2 = self.saat_esitlendi.clone();
        let mut goruntu = tokio::spawn(async move {
            let (kare_tx, mut kare_rx) = mpsc::channel::<Kare>(8);
            let c3 = c2.clone();
            let kabul = tokio::spawn(async move {
                while let Ok(mut akis) = c3.accept_uni().await {
                    let tx = kare_tx.clone();
                    tokio::spawn(async move {
                        if let Ok(Some(k)) = protokol::oku::<_, Kare>(&mut akis).await {
                            let _ = tx.send(k).await;
                        }
                    });
                }
            });
            let _kabul = AbortOnDrop(kabul);
            let mut b = Birlestirici::default();
            let mut sayac = 0u32;
            let mut son = std::time::Instant::now();
            let mut gecikmeler = VecDeque::<(std::time::Instant, u32)>::new();
            while let Some(k) = kare_rx.recv().await {
                b.uygula(&k)?;
                let simdi = std::time::Instant::now();
                if esit2.load(Ordering::Relaxed) {
                    gecikmeler.push_back((
                        simdi,
                        zaman::gecikme_ms(unix_ms(), fark2.load(Ordering::Relaxed), k.yakalama_ms),
                    ));
                }
                while gecikmeler
                    .front()
                    .is_some_and(|(t, _)| simdi.duration_since(*t) > Duration::from_secs(1))
                {
                    gecikmeler.pop_front();
                }
                sayac += 1;
                let _ = olay2.try_send(IzleyiciOlay::Kare {
                    genislik: b.goruntu.genislik,
                    yukseklik: b.goruntu.yukseklik,
                    rgba: b.goruntu.rgba.clone(),
                });
                if son.elapsed() >= Duration::from_secs(1) {
                    let fps = (sayac as f64 / son.elapsed().as_secs_f64()).round() as u32;
                    let (yol, rtt) = ag::secili_yol(&c2);
                    let rtt_ms = rtt.as_millis().min(u32::MAX as u128) as u32;
                    let mut ms: Vec<u32> = gecikmeler.iter().map(|(_, v)| *v).collect();
                    ms.sort_unstable();
                    let gecikme_ms = if ms.is_empty() { 0 } else { ms[ms.len() / 2] };
                    let _ = olay2
                        .send(IzleyiciOlay::Istatistik {
                            rtt_ms,
                            fps,
                            gecikme_ms,
                            yol: yol.ad().into(),
                        })
                        .await;
                    sayac = 0;
                    son = std::time::Instant::now();
                }
            }
            Ok::<_, anyhow::Error>(())
        });
        if izinler.pano && self.pano.es.is_none() {
            if let Some(p) = self.pano.kaynak.take() {
                self.pano.es = Some(tokio::task::block_in_place(|| Esitleyici::new(p)));
            }
        }
        let mut pano_saat = tokio::time::interval(PANO_ARALIGI);
        pano_saat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut saat_olcum = tokio::time::interval(Duration::from_secs(2));
        saat_olcum.tick().await;
        let mut son_kol: Option<Girdi> = None;
        let mut kol_yenile = tokio::time::interval(Duration::from_millis(100));
        kol_yenile.tick().await;
        const KOPTU: &str = "Bağlantı koptu.";
        const KAPANDI: &str = "Bağlantı kapandı.";
        let sonuc: String = loop {
            let pano = if izinler.pano { self.pano.es.as_mut() } else { None };
            let pano_var = pano.is_some();
            tokio::select! {
                s = &mut goruntu => break match s { Ok(Err(e)) => format!("Görüntü akışı kesildi: {e}"), _ => KAPANDI.into() },
                g = self.girdi.recv() => match g {
                    Some(g) => {
                        if let Girdi::Kol(k) = &g {
                            if !izinler.oyun_kolu { continue; }
                            son_kol = Some(g.clone());
                            if let Ok(payload) = bincode::serialize(k) {
                                if c.send_datagram(payload.into()).is_ok() { continue; }
                            }
                            if protokol::yaz(w, &Kontrol::Girdi(g)).await.is_err() { break KOPTU.into(); }
                            continue;
                        }
                        if !izinler.kontrol { continue; }
                        if protokol::yaz(w, &Kontrol::Girdi(g)).await.is_err() { break KOPTU.into(); }
                    }
                    None => break KAPANDI.into(),
                },
                Some(()) = self.kol_olay.recv() => { let _ = olay.send(IzleyiciOlay::KolAlgilandi).await; }
                _ = kol_yenile.tick(), if izinler.oyun_kolu => {
                    if let Some(g) = son_kol.clone() {
                        if protokol::yaz(w, &Kontrol::Girdi(g)).await.is_err() { break KOPTU.into(); }
                    }
                }
                _ = saat_olcum.tick() => {
                    let izleyici_ms = unix_ms();
                    if protokol::yaz(w, &Kontrol::SaatSor { izleyici_ms }).await.is_err() { break KOPTU.into(); }
                }
                m = gelen_rx.recv() => match m {
                    Some(Kontrol::Kapat(s)) => { *jeton = None; break s }
                    Some(Kontrol::DevamJetonu(j)) => { *jeton = Some(j); }
                    Some(Kontrol::SaatCevap { izleyici_ms, host_ms }) => {
                        let alim_ms = unix_ms();
                        self.saat_farki.store(zaman::saat_farki(izleyici_ms, host_ms, alim_ms), Ordering::Relaxed);
                        self.saat_esitlendi.store(true, Ordering::Relaxed);
                    }
                    Some(Kontrol::Pano(metin)) => {
                        if !izinler.pano || metin.is_empty() || !pano::sinir_icinde(&metin) { continue; }
                        if let Some(p) = pano { let _ = tokio::task::block_in_place(|| p.gelen(&metin)); }
                        let _ = olay.send(IzleyiciOlay::Pano(metin)).await;
                    }
                    Some(Kontrol::Titresim { slot, buyuk, kucuk }) => {
                        let _ = self.titresim.send((buyuk, kucuk));
                        let _ = olay.send(IzleyiciOlay::Titresim { slot, buyuk, kucuk }).await;
                    }
                    Some(_) => {}
                    None => break KOPTU.into(),
                },
                _ = pano_saat.tick(), if pano_var => {
                    let yeni = pano.and_then(|p| tokio::task::block_in_place(|| p.yokla()));
                    if let Some(metin) = yeni {
                        if protokol::yaz(w, &Kontrol::Pano(metin)).await.is_err() { break KOPTU.into(); }
                    }
                }
                _ = c.closed() => break KAPANDI.into(),
            }
        };
        okuyucu.abort();
        goruntu.abort();
        sonuc
    }
}

/// Görev tutamağı düşünce görevi durdurur.
struct AbortOnDrop(tokio::task::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn bekleme_artar_ve_sinirda_durur() {
        assert_eq!(bekleme(1), Duration::from_millis(500));
        assert_eq!(bekleme(2), Duration::from_millis(1_000));
        assert_eq!(bekleme(3), Duration::from_millis(2_000));
        assert_eq!(bekleme(4), Duration::from_millis(4_000));
        assert_eq!(bekleme(5), Duration::from_millis(5_000));
        assert_eq!(bekleme(50), Duration::from_millis(5_000));
    }

    fn davet(adresler: &[&str], relaylar: &[&str]) -> kod::Davet {
        kod::Davet {
            v: kod::DAVET_SURUMU,
            ad: "a".into(),
            adresler: adresler.iter().map(|s| s.to_string()).collect(),
            parmak_izi: String::new(),
            relaylar: relaylar.iter().map(|s| s.to_string()).collect(),
            bilet: String::new(),
            bitis: 0,
        }
    }

    #[test]
    fn yalniz_dongu_adresli_kod_disari_cikmaz() {
        assert_eq!(kurulum_sec(&davet(&["127.0.0.1:5"], &[])), Kurulum::YalnizYerel);
        assert_ne!(
            kurulum_sec(&davet(&["127.0.0.1:5", "192.168.1.2:5"], &[])),
            Kurulum::YalnizYerel
        );
        assert_ne!(
            kurulum_sec(&davet(&["127.0.0.1:5"], &["https://r.example/"])),
            Kurulum::YalnizYerel
        );
        assert_ne!(kurulum_sec(&davet(&[], &[])), Kurulum::YalnizYerel);
    }
}
