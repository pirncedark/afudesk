//! Bağlanan taraf: kodu çözer, doğrudan bağlanır, onayı bekler, kareleri birleştirir.
use crate::{
    ag,
    dosya::{self, GonderAyari, Gonderim},
    goruntu::Birlestirici,
    kod,
    protokol::{self, Girdi, Izinler, Kare, Kontrol, SURUM},
};
use anyhow::Result;
use std::{path::PathBuf, time::Duration};
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
    /// Dosya gönderme ilerlemesi. `gonderilen` devam noktası dahil toplam ilerlemedir.
    /// `bitti` ise `hata` boşsa başarılı.
    Dosya { ad: String, gonderilen: u64, toplam: u64, bitti: bool, hata: String },
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
    pub fn dosya_gonder(&self, yol: impl Into<PathBuf>) -> tokio::task::JoinHandle<Result<Gonderim>> {
        self.dosya_gonder_ayarli(yol.into(), GonderAyari::default())
    }

    pub(crate) fn dosya_gonder_ayarli(&self, yol: PathBuf, ayar: GonderAyari) -> tokio::task::JoinHandle<Result<Gonderim>> {
        let c = self.baglanti.clone();
        let olay = self.olay.upgrade();
        tokio::spawn(async move {
            let ad = yol.file_name().map(|a| a.to_string_lossy().into_owned()).unwrap_or_default();
            let mut son = (0, 0);
            let sonuc = dosya::gonder(&c, &yol, ayar, |gonderilen, toplam| {
                son = (gonderilen, toplam);
                // Ara ilerleme düşebilir (arayüz yavaşsa); sonuç asla düşmez.
                if let Some(o) = &olay {
                    let _ = o.try_send(IzleyiciOlay::Dosya { ad: ad.clone(), gonderilen, toplam, bitti: false, hata: String::new() });
                }
            })
            .await;
            if let Some(o) = &olay {
                let bitis = match &sonuc {
                    Ok(g) => IzleyiciOlay::Dosya { ad: ad.clone(), gonderilen: g.toplam, toplam: g.toplam, bitti: true, hata: String::new() },
                    Err(e) => IzleyiciOlay::Dosya { ad: ad.clone(), gonderilen: son.0, toplam: son.1, bitti: true, hata: e.to_string() },
                };
                let _ = o.send(bitis).await;
            }
            sonuc
        })
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
    let olay = olay_tx.downgrade();
    let c2 = c.clone();
    tokio::spawn(async move {
        let sebep = calis(&c2, &mut w, &mut r, &olay_tx, &mut girdi_rx).await;
        c2.close(0u32.into(), b"bitti");
        let _ = olay_tx.send(IzleyiciOlay::Koptu { sebep }).await;
    });
    Ok(Izleyici { olaylar, girdi, baglanti: c, olay })
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
