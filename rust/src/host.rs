//! Bağlantı veren taraf. Kendi QUIC sunucusunu açar, kodu üretir, gelen isteği
//! kullanıcı onayına sunar, onaylanırsa ekranı yayınlar ve (izin varsa) girdiyi uygular.
use crate::{
    adres::{self, UpnpDurum},
    ag, goruntu::Kodlayici,
    kod::{self, Davet},
    platform::Fabrika,
    protokol::{self, Izinler, Kontrol, SURUM},
};
use anyhow::Result;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::{mpsc, oneshot};

pub const VARSAYILAN_PORT: u16 = 47_470;
/// Kullanıcı bağlantı isteğine bu sürede yanıt vermezse istek reddedilir.
pub const ONAY_SURESI: Duration = Duration::from_secs(60);
pub const HEDEF_FPS: u64 = 15;
/// Aynı anda yolda olabilecek kare sayısı; aşılırsa en eski kare iptal edilir.
pub const AZAMI_UCUSTA: usize = 3;

/// Yayın ayarı: otomatik kalite döngüsü yazar, kodlayıcı iş parçacığı okur.
struct YayinAyari {
    kalite: std::sync::atomic::AtomicU8,
    fps: std::sync::atomic::AtomicU32,
}

/// Ağ durumuna göre kalite/FPS kararı (saf fonksiyon, test edilir).
/// `iptal`: son ölçüm aralığında yetişmediği için iptal edilen kare sayısı.
pub fn uyarla(kalite: u8, fps: u32, rtt_ms: u32, iptal: u32) -> (u8, u32) {
    if iptal > 0 || rtt_ms > 200 {
        (kalite.saturating_sub(10).max(35), fps.saturating_sub(3).max(8))
    } else if rtt_ms < 80 {
        ((kalite + 5).min(80), (fps + 2).min(24))
    } else {
        (kalite, fps)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HostOlay {
    Hazir { kod: String, parola: String, adresler: Vec<String>, erisim: String },
    Istek { ad: String },
    Baglandi { ad: String, izinler: Izinler },
    Koptu { sebep: String },
    Hata(String),
}

pub enum HostKomut {
    Kabul(Izinler),
    Red,
    Kes,
}

pub struct HostAyar {
    pub ad: String,
    /// 0 = VARSAYILAN_PORT'tan başlayarak boş port ara.
    pub port: u16,
    pub upnp: bool,
    /// Boşsa rastgele 6 hane.
    pub parola: String,
    /// Testler için: kodda yalnız 127.0.0.1 olsun, UPnP denenmesin.
    pub yalniz_yerel: bool,
}

pub struct Host {
    pub olaylar: mpsc::Receiver<HostOlay>,
    komut: mpsc::Sender<HostKomut>,
    durdur: Option<oneshot::Sender<()>>,
}

impl Host {
    pub async fn komut(&self, k: HostKomut) {
        let _ = self.komut.send(k).await;
    }
    /// Olay alıcısından bağımsız komut göndermek için (arayüz köprüsü).
    pub fn komut_gonderici(&self) -> mpsc::Sender<HostKomut> {
        self.komut.clone()
    }
    pub fn durdur(&mut self) {
        if let Some(d) = self.durdur.take() {
            let _ = d.send(());
        }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.durdur();
    }
}

fn bagla_port(kimlik: &ag::Kimlik, istek: u16) -> Result<quinn::Endpoint> {
    let adaylar: Vec<u16> = if istek == 0 {
        (VARSAYILAN_PORT..VARSAYILAN_PORT + 20).chain(std::iter::once(0)).collect()
    } else {
        vec![istek]
    };
    let mut son = None;
    for p in adaylar {
        match ag::sunucu(kimlik, SocketAddr::from(([0, 0, 0, 0], p))) {
            Ok(e) => return Ok(e),
            Err(e) => son = Some(e),
        }
    }
    Err(son.unwrap_or_else(|| anyhow::anyhow!("port açılamadı")))
}

pub async fn baslat(ayar: HostAyar, fabrika: Arc<dyn Fabrika>) -> Result<Host> {
    let (olay_tx, olaylar) = mpsc::channel(32);
    let (komut, komut_rx) = mpsc::channel(8);
    let (durdur_tx, durdur_rx) = oneshot::channel();
    let kimlik = ag::yeni_kimlik()?;
    let ep4 = bagla_port(&kimlik, ayar.port)?;
    let port = ep4.local_addr()?.port();
    // IPv6 aynı portta ayrı uç nokta (Windows'ta çift yığın varsayılan değil); açılamazsa önemsiz.
    let ep6 = ag::sunucu(&kimlik, SocketAddr::from(([0u16; 8], port))).ok();
    tokio::spawn(calis(ayar, fabrika, kimlik, ep4, ep6, olay_tx, komut_rx, durdur_rx));
    Ok(Host { olaylar, komut, durdur: Some(durdur_tx) })
}

async fn adresleri_topla(ayar: &HostAyar, port: u16) -> (Vec<String>, UpnpDurum, Option<adres::UpnpEslemesi>) {
    if ayar.yalniz_yerel {
        return (vec![format!("127.0.0.1:{port}")], UpnpDurum::Kapali, None);
    }
    let mut adresler = Vec::new();
    let yerel = adres::yerel_ipv4();
    if let Some(a) = yerel {
        adresler.push(format!("{a}:{port}"));
    }
    let (durum, eslesme) = match (ayar.upnp, yerel) {
        (true, Some(a)) => match adres::upnp_ac(a, port).await {
            Ok(e) => {
                adresler.push(e.dis_adres.to_string());
                (UpnpDurum::Acik(e.dis_adres.to_string()), Some(e))
            }
            Err(s) => (UpnpDurum::Yok(s), None),
        },
        _ => (UpnpDurum::Kapali, None),
    };
    for a6 in adres::genel_ipv6() {
        adresler.push(SocketAddr::from((a6, port)).to_string());
    }
    let durum = if matches!(durum, UpnpDurum::Acik(_)) || adres::genel_ipv6().is_empty() {
        durum
    } else {
        UpnpDurum::Acik("IPv6".into())
    };
    (adresler, durum, eslesme)
}

#[allow(clippy::too_many_arguments)]
async fn calis(
    ayar: HostAyar,
    fabrika: Arc<dyn Fabrika>,
    kimlik: ag::Kimlik,
    ep4: quinn::Endpoint,
    ep6: Option<quinn::Endpoint>,
    olay: mpsc::Sender<HostOlay>,
    mut komut: mpsc::Receiver<HostKomut>,
    mut durdur: oneshot::Receiver<()>,
) {
    let port = ep4.local_addr().map(|a| a.port()).unwrap_or(0);
    let (adresler, erisim, eslesme) = adresleri_topla(&ayar, port).await;
    if adresler.is_empty() {
        let _ = olay.send(HostOlay::Hata("Ağ bağlantısı bulunamadı.".into())).await;
        return;
    }
    let parola = if ayar.parola.trim().is_empty() { kod::yeni_parola() } else { ayar.parola.trim().to_owned() };
    'kod: loop {
        let bilet = kod::yeni_bilet();
        let davet = Davet {
            v: 2,
            ad: ayar.ad.clone(),
            adresler: adresler.clone(),
            parmak_izi: kimlik.parmak_izi.clone(),
            bilet: bilet.clone(),
            bitis: kod::simdi() + kod::GECERLILIK_SN,
        };
        let metin = match kod::kodla(&davet, &parola) {
            Ok(k) => k,
            Err(e) => {
                let _ = olay.send(HostOlay::Hata(e.to_string())).await;
                break;
            }
        };
        let _ = olay
            .send(HostOlay::Hazir { kod: metin, parola: parola.clone(), adresler: adresler.clone(), erisim: erisim.aciklama() })
            .await;
        let bitis = tokio::time::Instant::now() + Duration::from_secs(kod::GECERLILIK_SN as u64);
        loop {
            let gelen = tokio::select! {
                _ = &mut durdur => break 'kod,
                _ = tokio::time::sleep_until(bitis) => continue 'kod, // süre doldu: yeni kod
                g = ep4.accept() => g,
                g = async { match &ep6 { Some(e) => e.accept().await, None => std::future::pending().await } } => g,
            };
            let Some(gelen) = gelen else { break 'kod };
            let Ok(Ok(baglanti)) = tokio::time::timeout(Duration::from_secs(10), gelen).await else { continue };
            match oturum(&baglanti, &bilet, &fabrika, &olay, &mut komut, &mut durdur).await {
                Oturum::Reddedildi => continue,
                Oturum::Bitti(sebep) => {
                    let _ = olay.send(HostOlay::Koptu { sebep }).await;
                    continue 'kod; // bilet kullanıldı: yeni kod
                }
                Oturum::Durdur => break 'kod,
            }
        }
    }
    ep4.close(0u32.into(), b"kapandi");
    if let Some(e) = ep6 {
        e.close(0u32.into(), b"kapandi");
    }
    if let Some(e) = eslesme {
        e.kapat().await;
    }
}

enum Oturum {
    Reddedildi,
    Bitti(String),
    Durdur,
}

fn sabit_zamanli_esit(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

async fn reddet(w: &mut quinn::SendStream, c: &quinn::Connection, sebep: &str) {
    let _ = protokol::yaz(w, &Kontrol::Red(sebep.to_owned())).await;
    let _ = w.finish();
    // Karşı tarafın Red'i okuyabilmesi için kısa süre bekle.
    let _ = tokio::time::timeout(Duration::from_secs(2), c.closed()).await;
    c.close(1u32.into(), b"red");
}

async fn oturum(
    c: &quinn::Connection,
    bilet: &str,
    fabrika: &Arc<dyn Fabrika>,
    olay: &mpsc::Sender<HostOlay>,
    komut: &mut mpsc::Receiver<HostKomut>,
    durdur: &mut oneshot::Receiver<()>,
) -> Oturum {
    let Ok(Ok((mut w, mut r))) = tokio::time::timeout(Duration::from_secs(10), c.accept_bi()).await else {
        return Oturum::Reddedildi;
    };
    let merhaba = tokio::time::timeout(Duration::from_secs(10), protokol::oku::<_, Kontrol>(&mut r)).await;
    let Ok(Ok(Some(Kontrol::Merhaba { surum, bilet: gelen, ad }))) = merhaba else {
        c.close(1u32.into(), b"protokol");
        return Oturum::Reddedildi;
    };
    if surum != SURUM {
        reddet(&mut w, c, "AfuDesk sürümleri uyuşmuyor; iki taraf da güncellemeli.").await;
        return Oturum::Reddedildi;
    }
    if !sabit_zamanli_esit(&gelen, bilet) {
        reddet(&mut w, c, "Kod geçersiz ya da daha önce kullanılmış.").await;
        return Oturum::Reddedildi;
    }
    let ad: String = ad.chars().filter(|c| !c.is_control()).take(40).collect();
    let _ = olay.send(HostOlay::Istek { ad: ad.clone() }).await;
    // Bekleyen eski komutları at.
    while komut.try_recv().is_ok() {}
    let karar = tokio::select! {
        _ = &mut *durdur => return Oturum::Durdur,
        k = tokio::time::timeout(ONAY_SURESI, komut.recv()) => k,
    };
    let izinler = match karar {
        Ok(Some(HostKomut::Kabul(i))) => i,
        Ok(Some(HostKomut::Red)) | Ok(Some(HostKomut::Kes)) => {
            reddet(&mut w, c, "Karşı taraf bağlantıyı reddetti.").await;
            return Oturum::Reddedildi;
        }
        _ => {
            reddet(&mut w, c, "Karşı taraf zamanında yanıt vermedi.").await;
            return Oturum::Reddedildi;
        }
    };
    // Görüntü: yakalayıcı kendi iş parçacığında oluşturulur (platform tutamaçları Send değil).
    let (kare_tx, mut kare_rx) = mpsc::channel(2);
    let ayar = Arc::new(YayinAyari {
        kalite: std::sync::atomic::AtomicU8::new(70),
        fps: std::sync::atomic::AtomicU32::new(HEDEF_FPS as u32),
    });
    let ayar2 = ayar.clone();
    let (iptal_tx, iptal_rx) = std::sync::mpsc::channel::<Vec<(u32, u32)>>();
    let (boyut_tx, boyut_rx) = oneshot::channel::<Result<(u32, u32), String>>();
    let yayin_dur = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let dur2 = yayin_dur.clone();
    let fab2 = fabrika.clone();
    std::thread::spawn(move || {
        let ilk = fab2.yakalayici().and_then(|mut y| y.yakala().map(|g| (y, crate::goruntu::yayin_boyutu(g))));
        let (mut yakalayici, ilk) = match ilk {
            Ok(x) => x,
            Err(e) => {
                let _ = boyut_tx.send(Err(e.to_string()));
                return;
            }
        };
        let _ = boyut_tx.send(Ok((ilk.genislik, ilk.yukseklik)));
        let mut kodlayici = Kodlayici::new(70);
        let mut g = Some(ilk);
        while !dur2.load(std::sync::atomic::Ordering::Relaxed) {
            let t0 = std::time::Instant::now();
            let fps = ayar2.fps.load(std::sync::atomic::Ordering::Relaxed).max(1) as u64;
            let aralik = Duration::from_millis(1000 / fps);
            kodlayici.kalite_ayarla(ayar2.kalite.load(std::sync::atomic::Ordering::Relaxed));
            while let Ok(konumlar) = iptal_rx.try_recv() {
                kodlayici.yeniden_gonder(konumlar);
            }
            let goruntu = match g.take() {
                Some(x) => x,
                None => match yakalayici.yakala() {
                    Ok(x) => crate::goruntu::yayin_boyutu(x),
                    Err(_) => {
                        std::thread::sleep(aralik);
                        continue;
                    }
                },
            };
            if let Ok(Some(k)) = kodlayici.kodla(&goruntu) {
                if kare_tx.blocking_send(k).is_err() {
                    break;
                }
            }
            if let Some(kalan) = aralik.checked_sub(t0.elapsed()) {
                std::thread::sleep(kalan);
            }
        }
    });
    let (genislik, yukseklik) = match boyut_rx.await {
        Ok(Ok(b)) => b,
        Ok(Err(e)) => {
            reddet(&mut w, c, "Ekran yakalanamıyor.").await;
            return Oturum::Bitti(format!("Ekran yakalanamadı: {e}"));
        }
        Err(_) => {
            reddet(&mut w, c, "Ekran yakalanamıyor.").await;
            return Oturum::Bitti("Ekran yakalanamadı.".into());
        }
    };
    if protokol::yaz(&mut w, &Kontrol::Kabul { izinler: izinler.clone(), genislik, yukseklik }).await.is_err() {
        yayin_dur.store(true, std::sync::atomic::Ordering::Relaxed);
        return Oturum::Bitti("Bağlantı koptu.".into());
    }
    let _ = olay.send(HostOlay::Baglandi { ad, izinler: izinler.clone() }).await;

    // Her kare kendi tek yönlü akışında: bir karenin kaybı sonrakileri bekletmez.
    // Yolda çok kare birikirse en eskisi iptal edilir, döşemeleri yeniden gönderilir.
    let c2 = c.clone();
    let yayin = tokio::spawn(async move {
        struct Ucusta {
            konumlar: Vec<(u32, u32)>,
            iptal: Option<oneshot::Sender<()>>,
            gorev: tokio::task::JoinHandle<()>,
        }
        let mut ucusta: std::collections::VecDeque<Ucusta> = Default::default();
        let mut iptal_sayisi = 0u32;
        let mut olcum = tokio::time::interval(Duration::from_secs(1));
        olcum.tick().await;
        loop {
            tokio::select! {
                k = kare_rx.recv() => {
                    let Some(k) = k else { break };
                    ucusta.retain(|u| !u.gorev.is_finished());
                    while ucusta.len() >= AZAMI_UCUSTA {
                        let mut eski = ucusta.pop_front().expect("dolu");
                        if let Some(i) = eski.iptal.take() {
                            let _ = i.send(());
                        }
                        iptal_sayisi += 1;
                        let _ = iptal_tx.send(eski.konumlar);
                    }
                    let konumlar = k.dosemeler.iter().map(|d| (d.x, d.y)).collect();
                    let mut akis = c2.open_uni().await?;
                    let (iptal, mut iptal_al) = oneshot::channel::<()>();
                    let gorev = tokio::spawn(async move {
                        tokio::select! {
                            r = async { protokol::yaz(&mut akis, &k).await?; akis.finish()?; Ok::<_, anyhow::Error>(()) } => { let _ = r; }
                            _ = &mut iptal_al => { let _ = akis.reset(0u32.into()); }
                        }
                    });
                    ucusta.push_back(Ucusta { konumlar, iptal: Some(iptal), gorev });
                }
                _ = olcum.tick() => {
                    let rtt = c2.rtt().as_millis().min(u32::MAX as u128) as u32;
                    let (k, f) = uyarla(
                        ayar.kalite.load(std::sync::atomic::Ordering::Relaxed),
                        ayar.fps.load(std::sync::atomic::Ordering::Relaxed),
                        rtt,
                        iptal_sayisi,
                    );
                    ayar.kalite.store(k, std::sync::atomic::Ordering::Relaxed);
                    ayar.fps.store(f, std::sync::atomic::Ordering::Relaxed);
                    iptal_sayisi = 0;
                }
            }
        }
        for u in ucusta {
            u.gorev.abort();
        }
        Ok::<_, anyhow::Error>(())
    });

    let mut enjektor = if izinler.kontrol { fabrika.enjektor().ok() } else { None };
    let mut kol_siralari = std::collections::HashMap::<u8, u32>::new();
    let sonuc = loop {
        tokio::select! {
            _ = &mut *durdur => { c.close(0u32.into(), b"durdu"); break Oturum::Durdur; }
            k = komut.recv() => match k {
                Some(HostKomut::Kes) | None => {
                    let _ = protokol::yaz(&mut w, &Kontrol::Kapat("Karşı taraf bağlantıyı kesti.".into())).await;
                    let _ = w.finish();
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    c.close(0u32.into(), b"kes");
                    break Oturum::Bitti("Bağlantıyı kestin.".into());
                }
                _ => {}
            },
            m = protokol::oku::<_, Kontrol>(&mut r) => match m {
                Ok(Some(Kontrol::Girdi(g))) => {
                    if let protokol::Girdi::Kol(k) = &g {
                        if !izinler.oyun_kolu || !sira_yeni(kol_siralari.get(&k.slot).copied(), k.sira) { continue; }
                        kol_siralari.insert(k.slot, k.sira);
                    }
                    if let Some(e) = enjektor.as_mut() {
                        let _ = tokio::task::block_in_place(|| e.uygula(&g));
                    }
                }
                Ok(Some(Kontrol::Kapat(_))) | Ok(None) | Err(_) => break Oturum::Bitti("İzleyici bağlantıyı kapattı.".into()),
                Ok(Some(_)) => {}
            },
            d = c.read_datagram() => match d {
                Ok(b) => if let Ok(k) = bincode::deserialize::<protokol::KolDurumu>(&b) {
                    if izinler.oyun_kolu && sira_yeni(kol_siralari.get(&k.slot).copied(), k.sira) {
                        kol_siralari.insert(k.slot, k.sira);
                    }
                },
                Err(_) => {}
            },
        }
    };
    yayin_dur.store(true, std::sync::atomic::Ordering::Relaxed);
    yayin.abort();
    sonuc
}

/// RFC 1982 benzeri karşılaştırma; farkın yarı uzayı geçmediği varsayılır.
pub fn sira_yeni(eski: Option<u32>, yeni: u32) -> bool {
    eski.map(|e| yeni != e && yeni.wrapping_sub(e) < (1u32 << 31)).unwrap_or(true)
}

#[cfg(test)]
mod sira_testleri {
    use super::sira_yeni;
    #[test]
    fn sira_sarmali_ve_eski_paket() {
        assert!(sira_yeni(None, u32::MAX - 1));
        assert!(sira_yeni(Some(u32::MAX - 1), 1));
        assert!(!sira_yeni(Some(1), u32::MAX - 1));
        assert!(!sira_yeni(Some(7), 7));
        assert!(sira_yeni(Some(7), 8));
    }
}

#[cfg(test)]
mod uyarlama_testleri {
    use super::uyarla;

    #[test]
    fn kotu_agda_kalite_ve_fps_duser_sinirlarda_durur() {
        assert_eq!(uyarla(70, 15, 250, 0), (60, 12));
        assert_eq!(uyarla(70, 15, 30, 2), (60, 12), "iptal varsa düşer");
        assert_eq!(uyarla(36, 9, 300, 5), (35, 8), "alt sınır");
    }

    #[test]
    fn iyi_agda_yukselir_ust_sinirda_durur() {
        assert_eq!(uyarla(70, 15, 20, 0), (75, 17));
        assert_eq!(uyarla(79, 23, 20, 0), (80, 24));
        assert_eq!(uyarla(80, 24, 20, 0), (80, 24));
    }

    #[test]
    fn orta_agda_sabit() {
        assert_eq!(uyarla(60, 12, 120, 0), (60, 12));
    }
}
