//! Bağlanan taraf: kodu çözer, doğrudan bağlanır, onayı bekler, kareleri birleştirir.
use crate::{
    ag, goruntu::Birlestirici,
    kod,
    protokol::{self, Girdi, Izinler, Kare, Kontrol, SURUM},
};
use anyhow::Result;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq)]
pub enum IzleyiciOlay {
    /// Bağlandı, karşı tarafın onayı bekleniyor.
    OnayBekleniyor { karsi_ad: String },
    Kabul { izinler: Izinler, genislik: u32, yukseklik: u32 },
    /// Birleştirilmiş tam ekran (RGBA).
    Kare { genislik: u32, yukseklik: u32, rgba: Vec<u8> },
    /// Saniyede bir: bağlantı gidiş-dönüş süresi ve ekrana gelen kare hızı.
    Istatistik { rtt_ms: u32, fps: u32 },
    Koptu { sebep: String },
    KolAlgilandi,
    Titresim { slot: u8, buyuk: u8, kucuk: u8 },
}

pub struct Izleyici {
    pub olaylar: mpsc::Receiver<IzleyiciOlay>,
    girdi: mpsc::Sender<Girdi>,
    baglanti: quinn::Connection,
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
}

pub async fn baglan(kod_metni: &str, parola: &str, ad: &str) -> Result<Izleyici> {
    let d = kod::coz(kod_metni, parola, kod::simdi())?;
    let c = ag::baglan(&d.adresler, &d.parmak_izi, Duration::from_secs(6)).await?;
    let (mut w, mut r) = c.open_bi().await?;
    protokol::yaz(&mut w, &Kontrol::Merhaba { surum: SURUM, bilet: d.bilet.clone(), ad: ad.to_owned() }).await?;
    let (olay_tx, olaylar) = mpsc::channel(4);
    let (girdi, mut girdi_rx) = mpsc::channel::<Girdi>(256);
    let (kol_olay_tx, mut kol_olay_rx) = mpsc::unbounded_channel();
    #[cfg(all(not(target_os = "android"), not(test)))]
    {
        let tx = girdi.clone();
        std::thread::spawn(move || crate::kol::oku(tx, kol_olay_tx));
    }
    #[cfg(any(target_os = "android", test))]
    drop(kol_olay_tx);
    let _ = olay_tx.send(IzleyiciOlay::OnayBekleniyor { karsi_ad: d.ad.clone() }).await;
    let c2 = c.clone();
    tokio::spawn(async move {
        let sebep = calis(&c2, &mut w, &mut r, &olay_tx, &mut girdi_rx, &mut kol_olay_rx).await;
        c2.close(0u32.into(), b"bitti");
        let _ = olay_tx.send(IzleyiciOlay::Koptu { sebep }).await;
    });
    Ok(Izleyici { olaylar, girdi, baglanti: c })
}

async fn calis(
    c: &quinn::Connection,
    w: &mut quinn::SendStream,
    r: &mut quinn::RecvStream,
    olay: &mpsc::Sender<IzleyiciOlay>,
    girdi: &mut mpsc::Receiver<Girdi>,
    kol_olay: &mut mpsc::UnboundedReceiver<()>,
) -> String {
    // Onay: karşı taraf 60 sn içinde karar verir.
    let ilk = tokio::time::timeout(Duration::from_secs(75), protokol::oku::<_, Kontrol>(r)).await;
    let izinler = match ilk {
        Ok(Ok(Some(Kontrol::Kabul { izinler, genislik, yukseklik }))) => {
            let _ = olay.send(IzleyiciOlay::Kabul { izinler: izinler.clone(), genislik, yukseklik }).await;
            izinler
        }
        Ok(Ok(Some(Kontrol::Red(s)))) => return s,
        Err(_) => return "Karşı taraf yanıt vermedi.".into(),
        _ => return "Bağlantı koptu.".into(),
    };
    let c2 = c.clone();
    let olay2 = olay.clone();
    let mut goruntu = tokio::spawn(async move {
        // Her kare ayrı akışta gelir; akışlar paralel okunur, birleştirici sırayı korur.
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
        while let Some(k) = kare_rx.recv().await {
            b.uygula(&k)?;
            sayac += 1;
            // Arayüz yavaşsa kare düşür (en yenisi önemli).
            let _ = olay2.try_send(IzleyiciOlay::Kare {
                genislik: b.goruntu.genislik,
                yukseklik: b.goruntu.yukseklik,
                rgba: b.goruntu.rgba.clone(),
            });
            if son.elapsed() >= Duration::from_secs(1) {
                let fps = (sayac as f64 / son.elapsed().as_secs_f64()).round() as u32;
                let rtt_ms = c2.rtt().as_millis().min(u32::MAX as u128) as u32;
                let _ = olay2.send(IzleyiciOlay::Istatistik { rtt_ms, fps }).await;
                sayac = 0;
                son = std::time::Instant::now();
            }
        }
        Ok::<_, anyhow::Error>(())
    });
    let mut son_kol: Option<Girdi> = None;
    let mut kol_yenile = tokio::time::interval(Duration::from_millis(100));
    kol_yenile.tick().await;
    loop {
        tokio::select! {
            s = &mut goruntu => {
                return match s { Ok(Err(e)) => format!("Görüntü akışı kesildi: {e}"), _ => "Bağlantı kapandı.".into() };
            }
            g = girdi.recv() => match g {
                Some(g) => {
                    if !izinler.kontrol { continue; }
                    if let Girdi::Kol(k) = &g {
                        son_kol = Some(g.clone());
                        let mut paket = Vec::with_capacity(32);
                        if let Ok(b) = bincode::serialize(k) {
                            paket.extend_from_slice(&b);
                        }
                        if !paket.is_empty() && c.send_datagram(paket.into()).is_ok() { continue; }
                    }
                    if protokol::yaz(w, &Kontrol::Girdi(g)).await.is_err() { return "Bağlantı koptu.".into(); }
                }
                None => return "Bağlantı kapandı.".into(),
            },
            Some(()) = kol_olay.recv() => { let _ = olay.send(IzleyiciOlay::KolAlgilandi).await; }
            _ = kol_yenile.tick() => {
                if izinler.kontrol {
                    if let Some(g) = son_kol.clone() {
                        if protokol::yaz(w, &Kontrol::Girdi(g)).await.is_err() { return "Bağlantı koptu.".into(); }
                    }
                }
            }
            m = protokol::oku::<_, Kontrol>(r) => match m {
                Ok(Some(Kontrol::Kapat(s))) => return s,
                Ok(Some(Kontrol::Titresim { slot, buyuk, kucuk })) => { let _ = olay.send(IzleyiciOlay::Titresim { slot, buyuk, kucuk }).await; }
                Ok(Some(_)) => {}
                _ => return "Bağlantı koptu.".into(),
            },
            _ = c.closed() => return "Bağlantı kapandı.".into(),
        }
    }
}
