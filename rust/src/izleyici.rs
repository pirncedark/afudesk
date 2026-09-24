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
    Koptu { sebep: String },
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
    let _ = olay_tx.send(IzleyiciOlay::OnayBekleniyor { karsi_ad: d.ad.clone() }).await;
    let c2 = c.clone();
    tokio::spawn(async move {
        let sebep = calis(&c2, &mut w, &mut r, &olay_tx, &mut girdi_rx).await;
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
        let mut akis = c2.accept_uni().await?;
        let mut b = Birlestirici::default();
        while let Some(k) = protokol::oku::<_, Kare>(&mut akis).await? {
            b.uygula(&k)?;
            // Arayüz yavaşsa kare düşür (en yenisi önemli).
            let _ = olay2.try_send(IzleyiciOlay::Kare {
                genislik: b.goruntu.genislik,
                yukseklik: b.goruntu.yukseklik,
                rgba: b.goruntu.rgba.clone(),
            });
        }
        Ok::<_, anyhow::Error>(())
    });
    loop {
        tokio::select! {
            s = &mut goruntu => {
                return match s { Ok(Err(e)) => format!("Görüntü akışı kesildi: {e}"), _ => "Bağlantı kapandı.".into() };
            }
            g = girdi.recv() => match g {
                Some(g) => {
                    if !izinler.kontrol { continue; }
                    if protokol::yaz(w, &Kontrol::Girdi(g)).await.is_err() { return "Bağlantı koptu.".into(); }
                }
                None => return "Bağlantı kapandı.".into(),
            },
            m = protokol::oku::<_, Kontrol>(r) => match m {
                Ok(Some(Kontrol::Kapat(s))) => return s,
                Ok(Some(_)) => {}
                _ => return "Bağlantı koptu.".into(),
            },
            _ = c.closed() => return "Bağlantı kapandı.".into(),
        }
    }
}
