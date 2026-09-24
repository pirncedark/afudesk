//! Bağlanan taraf: kodu çözer, doğrudan bağlanır, onayı bekler, kareleri birleştirir.
use crate::{
    ag,
    dosya::{self, GonderAyari, Gonderim},
    goruntu::Birlestirici,
    kod,
    pano::{self, Esitleyici, Pano, PANO_ARALIGI},
    protokol::{self, Girdi, Izinler, Kare, Kontrol, SURUM},
    zaman,
};
use anyhow::Result;
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicI64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::mpsc;

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
    /// Saniyede bir: bağlantı gidiş-dönüş süresi ve ekrana gelen kare hızı.
    Istatistik {
        rtt_ms: u32,
        fps: u32,
        gecikme_ms: u32,
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
    baglanti: quinn::Connection,
    /// Zayıf: oturum bitince olay kanalı kapanabilsin.
    olay: mpsc::WeakSender<IzleyiciOlay>,
}

impl Izleyici {
    /// Girdi kuyruğu doluysa (yavaş ağ) fare hareketleri düşürülür, diğerleri beklenir.
    pub async fn gonder(&self, g: Girdi) {
        if matches!(g, Girdi::FareKonum { .. }) {
            let _ = self.girdi.try_send(g);
        } else {
            let _ = self.girdi.send(g).await;
        }
    }
    pub fn kapat(&self) {
        self.baglanti.close(0u32.into(), b"izleyici");
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
        let c = self.baglanti.clone();
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

/// `pano`: izleyicinin kendi panosu. `None` (ör. Android) ise yalnız host → izleyici yönü
/// çalışır; gelen metin `IzleyiciOlay::Pano` ile arayüze bildirilir.
pub async fn baglan_panolu(
    kod_metni: &str,
    parola: &str,
    ad: &str,
    pano: Option<Box<dyn Pano>>,
) -> Result<Izleyici> {
    let d = kod::coz(kod_metni, parola, kod::simdi())?;
    let c = ag::baglan(&d.adresler, &d.parmak_izi, Duration::from_secs(6)).await?;
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
    let (girdi, mut girdi_rx) = mpsc::channel::<Girdi>(256);
    let _ = olay_tx
        .send(IzleyiciOlay::OnayBekleniyor {
            karsi_ad: d.ad.clone(),
        })
        .await;
    let olay = olay_tx.downgrade();

    let (kol_olay_tx, mut kol_olay_rx) = mpsc::unbounded_channel();
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
    let c2 = c.clone();
    let saat_farki = Arc::new(AtomicI64::new(0));
    let saat_esitlendi = Arc::new(AtomicBool::new(false));
    let fark2 = saat_farki.clone();
    let esit2 = saat_esitlendi.clone();
    tokio::spawn(async move {
        let sebep = calis(
            &c2,
            &mut w,
            r,
            &olay_tx,
            &mut girdi_rx,
            pano,
            fark2,
            esit2,
            &mut kol_olay_rx,
            titresim_tx,
        )
        .await;
        c2.close(0u32.into(), b"bitti");
        let _ = olay_tx.send(IzleyiciOlay::Koptu { sebep }).await;
    });
    Ok(Izleyici {
        olaylar,
        girdi,
        baglanti: c,
        olay,
    })
}

async fn calis(
    c: &quinn::Connection,
    w: &mut quinn::SendStream,
    mut r: quinn::RecvStream,
    olay: &mpsc::Sender<IzleyiciOlay>,
    girdi: &mut mpsc::Receiver<Girdi>,
    pano: Option<Box<dyn Pano>>,
    saat_farki: Arc<AtomicI64>,
    saat_esitlendi: Arc<AtomicBool>,
    kol_olay: &mut mpsc::UnboundedReceiver<()>,
    titresim: std::sync::mpsc::Sender<(u8, u8)>,
) -> String {
    let ilk =
        tokio::time::timeout(Duration::from_secs(75), protokol::oku::<_, Kontrol>(&mut r)).await;
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
        Ok(Ok(Some(Kontrol::Red(s)))) => return s,
        Err(_) => return "Kar?? taraf yan?t vermedi.".into(),
        _ => return "Ba?lant? koptu.".into(),
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
    let fark2 = saat_farki.clone();
    let esit2 = saat_esitlendi.clone();
    let mut goruntu = tokio::spawn(async move {
        let (kare_tx, mut kare_rx) = mpsc::channel::<Kare>(8);
        let c3 = c2.clone();
        tokio::spawn(async move {
            while let Ok(mut akis) = c3.accept_uni().await {
                let tx = kare_tx.clone();
                tokio::spawn(async move {
                    if let Ok(Some(k)) = protokol::oku::<_, Kare>(&mut akis).await {
                        let _ = tx.send(k).await;
                    }
                });
            }
        });
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
                let rtt_ms = c2.rtt().as_millis().min(u32::MAX as u128) as u32;
                let mut ms: Vec<u32> = gecikmeler.iter().map(|(_, v)| *v).collect();
                ms.sort_unstable();
                let gecikme_ms = if ms.is_empty() { 0 } else { ms[ms.len() / 2] };
                let _ = olay2
                    .send(IzleyiciOlay::Istatistik {
                        rtt_ms,
                        fps,
                        gecikme_ms,
                    })
                    .await;
                sayac = 0;
                son = std::time::Instant::now();
            }
        }
        Ok::<_, anyhow::Error>(())
    });
    let mut pano = match pano {
        Some(p) if izinler.pano => Some(tokio::task::block_in_place(|| Esitleyici::new(p))),
        _ => None,
    };
    let mut pano_saat = tokio::time::interval(PANO_ARALIGI);
    pano_saat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut saat_olcum = tokio::time::interval(Duration::from_secs(2));
    saat_olcum.tick().await;
    let mut son_kol: Option<Girdi> = None;
    let mut kol_yenile = tokio::time::interval(Duration::from_millis(100));
    kol_yenile.tick().await;
    let sonuc = loop {
        tokio::select! {
            s = &mut goruntu => break match s { Ok(Err(e)) => format!("G?r?nt? ak??? kesildi: {e}"), _ => "Ba?lant? kapand?.".into() },
            g = girdi.recv() => match g {
                Some(g) => {
                    if let Girdi::Kol(k) = &g {
                        if !izinler.oyun_kolu { continue; }
                        son_kol = Some(g.clone());
                        if let Ok(payload) = bincode::serialize(k) {
                            if c.send_datagram(payload.into()).is_ok() { continue; }
                        }
                        if protokol::yaz(w, &Kontrol::Girdi(g)).await.is_err() { break "Ba?lant? koptu.".into(); }
                        continue;
                    }
                    if !izinler.kontrol { continue; }
                    if protokol::yaz(w, &Kontrol::Girdi(g)).await.is_err() { break "Ba?lant? koptu.".into(); }
                }
                None => break "Ba?lant? kapand?.".into(),
            },
            Some(()) = kol_olay.recv() => { let _ = olay.send(IzleyiciOlay::KolAlgilandi).await; }
            _ = kol_yenile.tick(), if izinler.oyun_kolu => {
                if let Some(g) = son_kol.clone() {
                    if protokol::yaz(w, &Kontrol::Girdi(g)).await.is_err() { break "Ba?lant? koptu.".into(); }
                }
            }
            _ = saat_olcum.tick() => {
                let izleyici_ms = unix_ms();
                if protokol::yaz(w, &Kontrol::SaatSor { izleyici_ms }).await.is_err() { break "Ba?lant? koptu.".into(); }
            }
            m = gelen_rx.recv() => match m {
                Some(Kontrol::Kapat(s)) => break s,
                Some(Kontrol::SaatCevap { izleyici_ms, host_ms }) => {
                    let alim_ms = unix_ms();
                    saat_farki.store(zaman::saat_farki(izleyici_ms, host_ms, alim_ms), Ordering::Relaxed);
                    saat_esitlendi.store(true, Ordering::Relaxed);
                }
                Some(Kontrol::Pano(metin)) => {
                    if !izinler.pano || metin.is_empty() || !pano::sinir_icinde(&metin) { continue; }
                    if let Some(p) = pano.as_mut() { let _ = tokio::task::block_in_place(|| p.gelen(&metin)); }
                    let _ = olay.send(IzleyiciOlay::Pano(metin)).await;
                }
                Some(Kontrol::Titresim { slot, buyuk, kucuk }) => {
                    let _ = titresim.send((buyuk, kucuk));
                    let _ = olay.send(IzleyiciOlay::Titresim { slot, buyuk, kucuk }).await;
                }
                Some(_) => {}
                None => break "Ba?lant? koptu.".into(),
            },
            _ = pano_saat.tick(), if pano.is_some() => {
                let yeni = pano.as_mut().and_then(|p| tokio::task::block_in_place(|| p.yokla()));
                if let Some(metin) = yeni {
                    if protokol::yaz(w, &Kontrol::Pano(metin)).await.is_err() { break "Ba?lant? koptu.".into(); }
                }
            }
            _ = c.closed() => break "Ba?lant? kapand?.".into(),
        }
    };
    okuyucu.abort();
    sonuc
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}
