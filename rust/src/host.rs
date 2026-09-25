//! Bağlantı veren taraf. iroh uç noktası açar (doğrudan + NAT delme + relay yedeği),
//! kodu üretir, gelen isteği kullanıcı onayına sunar, onaylanırsa ekranı yayınlar ve
//! (izin varsa) girdiyi uygular, panoyu karşı tarafla paylaşır. Bağlantı istemeden
//! koparsa aynı izleyici `DEVAM_SURESI` içinde onay sorulmadan geri dönebilir.
use crate::{
    ag::{self, Baglanti, GonderAkisi, Kurulum},
    dosya,
    goruntu::Kodlayici,
    kod::{self, Davet},
    pano::{Esitleyici, PANO_ARALIGI},
    platform::Fabrika,
    protokol::{self, Izinler, Kontrol, SURUM},
};
use anyhow::Result;
use iroh::{Endpoint, EndpointId};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};

/// Kullanıcı bağlantı isteğine bu sürede yanıt vermezse istek reddedilir.
pub const ONAY_SURESI: Duration = Duration::from_secs(60);
/// İstemeden kopan oturuma izleyicinin onaysız geri dönebileceği süre.
pub const DEVAM_SURESI: Duration = Duration::from_secs(60);
/// Kod üretmeden önce relay'e bağlanmak için beklenen en uzun süre.
const CEVRIMICI_BEKLE: Duration = Duration::from_secs(8);
pub const HEDEF_FPS: u64 = 15;
/// Aynı anda yolda olabilecek kare sayısı; aşılırsa en eski kare iptal edilir.
pub const AZAMI_UCUSTA: usize = 3;
/// Uygulama kapanış kodu: "ağ değişti, geri döneceğim" — oturum devam bekler.
pub const KOPUS_KODU: u32 = 2;

/// Yayın ayarı: otomatik kalite döngüsü yazar, kodlayıcı iş parçacığı okur.
struct YayinAyari {
    kalite: std::sync::atomic::AtomicU8,
    fps: std::sync::atomic::AtomicU32,
}

/// Ağ durumuna göre kalite/FPS kararı (saf fonksiyon, test edilir).
/// `iptal`: son ölçüm aralığında yetişmediği için iptal edilen kare sayısı.
pub fn uyarla(kalite: u8, fps: u32, rtt_ms: u32, iptal: u32) -> (u8, u32) {
    if iptal > 0 || rtt_ms > 200 {
        (
            kalite.saturating_sub(10).max(35),
            fps.saturating_sub(3).max(8),
        )
    } else if rtt_ms < 80 {
        ((kalite + 5).min(80), (fps + 2).min(24))
    } else {
        (kalite, fps)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HostOlay {
    Hazir {
        kod: String,
        parola: String,
        adresler: Vec<String>,
        erisim: String,
    },
    Istek {
        ad: String,
    },
    Baglandi {
        ad: String,
        izinler: Izinler,
    },
    Koptu {
        sebep: String,
    },
    Hata(String),
    /// Dosya alındı.
    DosyaAlindi {
        ad: String,
        yol: String,
    },
    Uyari(String),
}

pub enum HostKomut {
    Kabul(Izinler),
    Red,
    Kes,
}

pub struct HostAyar {
    pub ad: String,
    /// Modemde UPnP/PCP/NAT-PMP ile port açmayı dene (doğrudan yol şansını artırır).
    pub upnp: bool,
    /// Boşsa rastgele 6 hane.
    pub parola: String,
    /// Testler için: relay yok, kodda yalnız 127.0.0.1 olsun, dışarı istek çıkmasın.
    pub yalniz_yerel: bool,
    /// Alınan dosyaların klasörü; `None` = İndirilenler\AfuDesk.
    pub dosya_klasoru: Option<PathBuf>,
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

pub async fn baslat(ayar: HostAyar, fabrika: Arc<dyn Fabrika>) -> Result<Host> {
    let (olay_tx, olaylar) = mpsc::channel(32);
    let (komut, komut_rx) = mpsc::channel(8);
    let (durdur_tx, durdur_rx) = oneshot::channel();
    let kurulum = if ayar.yalniz_yerel {
        Kurulum::YalnizYerel
    } else {
        Kurulum::ortamdan()
    };
    let ep = ag::uc_nokta(kurulum, ayar.upnp).await?;
    tokio::spawn(calis(
        ayar, fabrika, kurulum, ep, olay_tx, komut_rx, durdur_rx,
    ));
    Ok(Host {
        olaylar,
        komut,
        durdur: Some(durdur_tx),
    })
}

/// Koda yazılacak doğrudan adresler ve relay adresleri.
fn adresleri_topla(ep: &Endpoint, kurulum: Kurulum) -> (Vec<String>, Vec<String>) {
    match kurulum {
        Kurulum::YalnizYerel => (
            ep.bound_sockets()
                .iter()
                .filter(|a| a.ip().is_loopback())
                .map(|a| a.to_string())
                .collect(),
            vec![],
        ),
        _ => ag::yayinlanacak(ep),
    }
}

pub fn erisim_aciklamasi(relay_var: bool, yalniz_yerel: bool) -> String {
    if yalniz_yerel {
        "Yalnız bu bilgisayardan (test)".into()
    } else if relay_var {
        "İnternetten ulaşılabilir: önce doğrudan bağlantı denenir, olmazsa relay üzerinden".into()
    } else {
        "Yalnız aynı ağdan ulaşılabilir — relay sunucusuna ulaşılamadı (internet var mı?)".into()
    }
}

/// İzleyicinin onaysız geri dönebilmesi için saklanan oturum bilgisi.
#[derive(Clone)]
struct DevamBilgisi {
    jeton: String,
    kimlik: EndpointId,
    izinler: Izinler,
    ad: String,
}

enum Giris<'a> {
    /// Koddaki tek kullanımlık bilet.
    Yeni(&'a str),
    Devam(&'a DevamBilgisi),
}

#[allow(clippy::too_many_arguments)]
async fn calis(
    ayar: HostAyar,
    fabrika: Arc<dyn Fabrika>,
    kurulum: Kurulum,
    ep: Endpoint,
    olay: mpsc::Sender<HostOlay>,
    mut komut: mpsc::Receiver<HostKomut>,
    mut durdur: oneshot::Receiver<()>,
) {
    if kurulum != Kurulum::YalnizYerel {
        ag::cevrimici(&ep, CEVRIMICI_BEKLE).await;
    }
    let klasor = ayar
        .dosya_klasoru
        .clone()
        .unwrap_or_else(dosya::varsayilan_klasor);
    let parola = if ayar.parola.trim().is_empty() {
        kod::yeni_parola()
    } else {
        ayar.parola.trim().to_owned()
    };
    let mut devam: Option<DevamBilgisi> = None;

    'kod: loop {
        // Kopan oturum: yeni kod üretmeden önce izleyicinin geri dönmesini bekle.
        if let Some(d) = devam.take() {
            let _ = olay
                .send(HostOlay::Uyari(
                    "Bağlantı koptu; karşı taraf yeniden bağlanıyor…".into(),
                ))
                .await;
            let bitis = tokio::time::Instant::now() + DEVAM_SURESI;
            let sonuc = loop {
                let gelen = tokio::select! {
                    _ = &mut durdur => break 'kod,
                    _ = tokio::time::sleep_until(bitis) => break None,
                    g = ep.accept() => g,
                };
                let Some(gelen) = gelen else { break 'kod };
                let Ok(Ok(baglanti)) = tokio::time::timeout(Duration::from_secs(10), gelen).await
                else {
                    continue;
                };
                match oturum(
                    &baglanti,
                    Giris::Devam(&d),
                    &fabrika,
                    &klasor,
                    &olay,
                    &mut komut,
                    &mut durdur,
                )
                .await
                {
                    Oturum::Reddedildi => continue,
                    s => break Some(s),
                }
            };
            match sonuc {
                Some(Oturum::Durdur) => break 'kod,
                Some(Oturum::Koptu(d2)) => {
                    devam = Some(d2);
                    continue 'kod;
                }
                Some(Oturum::Bitti(sebep)) => {
                    let _ = olay.send(HostOlay::Koptu { sebep }).await;
                }
                Some(Oturum::Reddedildi) | None => {
                    let _ = olay
                        .send(HostOlay::Koptu {
                            sebep: "Bağlantı koptu; karşı taraf geri dönmedi.".into(),
                        })
                        .await;
                }
            }
        }

        let (adresler, relaylar) = adresleri_topla(&ep, kurulum);
        if adresler.is_empty() && relaylar.is_empty() {
            let _ = olay
                .send(HostOlay::Hata("Ağ bağlantısı bulunamadı.".into()))
                .await;
            break;
        }
        let bilet = kod::yeni_bilet();
        let davet = Davet {
            v: kod::DAVET_SURUMU,
            ad: ayar.ad.clone(),
            adresler: adresler.clone(),
            parmak_izi: ag::kimlik_metni(ep.id()),
            relaylar: relaylar.clone(),
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
            .send(HostOlay::Hazir {
                kod: metin,
                parola: parola.clone(),
                adresler: adresler.clone(),
                erisim: erisim_aciklamasi(!relaylar.is_empty(), kurulum == Kurulum::YalnizYerel),
            })
            .await;
        let bitis = tokio::time::Instant::now() + Duration::from_secs(kod::GECERLILIK_SN as u64);
        loop {
            let gelen = tokio::select! {
                _ = &mut durdur => break 'kod,
                _ = tokio::time::sleep_until(bitis) => continue 'kod, // süre doldu: yeni kod
                g = ep.accept() => g,
            };
            let Some(gelen) = gelen else { break 'kod };
            let Ok(Ok(baglanti)) = tokio::time::timeout(Duration::from_secs(10), gelen).await
            else {
                continue;
            };
            match oturum(
                &baglanti,
                Giris::Yeni(&bilet),
                &fabrika,
                &klasor,
                &olay,
                &mut komut,
                &mut durdur,
            )
            .await
            {
                Oturum::Reddedildi => continue,
                Oturum::Bitti(sebep) => {
                    let _ = olay.send(HostOlay::Koptu { sebep }).await;
                    continue 'kod; // bilet kullanıldı: yeni kod
                }
                Oturum::Koptu(d) => {
                    devam = Some(d);
                    continue 'kod;
                }
                Oturum::Durdur => break 'kod,
            }
        }
    }
    ep.close().await;
}

enum Oturum {
    Reddedildi,
    Bitti(String),
    /// Bağlantı istemeden koptu; izleyici geri dönebilir.
    Koptu(DevamBilgisi),
    Durdur,
}

/// Bağlantı ağ yüzünden mi koptu (zaman aşımı, sıfırlama) yoksa karşı taraf mı kapattı?
/// Karşı taraf `KOPUS_KODU` ile kapattıysa ("ağ değişti, döneceğim") da istemsiz sayılır.
pub(crate) fn istemsiz_kopus(c: &Baglanti) -> bool {
    use iroh::endpoint::ConnectionError as H;
    match c.close_reason() {
        Some(H::ApplicationClosed(k)) => k.error_code == KOPUS_KODU.into(),
        Some(H::LocallyClosed) | None => false,
        Some(_) => true,
    }
}

fn sabit_zamanli_esit(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

async fn reddet(w: &mut GonderAkisi, c: &Baglanti, sebep: &str) {
    let _ = protokol::yaz(w, &Kontrol::Red(sebep.to_owned())).await;
    let _ = w.finish();
    // Karşı tarafın Red'i okuyabilmesi için kısa süre bekle.
    let _ = tokio::time::timeout(Duration::from_secs(2), c.closed()).await;
    c.close(1u32.into(), b"red");
}

async fn oturum(
    c: &Baglanti,
    giris: Giris<'_>,
    fabrika: &Arc<dyn Fabrika>,
    klasor: &Path,
    olay: &mpsc::Sender<HostOlay>,
    komut: &mut mpsc::Receiver<HostKomut>,
    durdur: &mut oneshot::Receiver<()>,
) -> Oturum {
    let Ok(Ok((mut w, mut r))) = tokio::time::timeout(Duration::from_secs(10), c.accept_bi()).await
    else {
        return Oturum::Reddedildi;
    };
    let ilk =
        tokio::time::timeout(Duration::from_secs(10), protokol::oku::<_, Kontrol>(&mut r)).await;
    const SURUM_FARKLI: &str = "AfuDesk sürümleri uyuşmuyor; iki taraf da güncellemeli.";
    const OTURUM_BITTI: &str = "Oturum sona ermiş; yeni kod iste.";
    const KOD_GECERSIZ: &str = "Kod geçersiz ya da daha önce kullanılmış.";
    // (ad, önceden verilmiş izinler): devam eden oturumda onay yeniden sorulmaz.
    let (ad, onceki) = match (ilk, &giris) {
        (Ok(Ok(Some(Kontrol::Merhaba { surum, bilet: gelen, ad }))), Giris::Yeni(bilet)) => {
            if surum != SURUM {
                reddet(&mut w, c, SURUM_FARKLI).await;
                return Oturum::Reddedildi;
            }
            if !sabit_zamanli_esit(&gelen, bilet) {
                reddet(&mut w, c, KOD_GECERSIZ).await;
                return Oturum::Reddedildi;
            }
            (ad, None)
        }
        (Ok(Ok(Some(Kontrol::Devam { surum, jeton, .. }))), Giris::Devam(d)) => {
            if surum != SURUM {
                reddet(&mut w, c, SURUM_FARKLI).await;
                return Oturum::Reddedildi;
            }
            // Jeton + cihaz kimliği birlikte: jetonu ele geçiren başka cihaz dönemez.
            if !sabit_zamanli_esit(&jeton, &d.jeton) || c.remote_id() != d.kimlik {
                reddet(&mut w, c, OTURUM_BITTI).await;
                return Oturum::Reddedildi;
            }
            (d.ad.clone(), Some(d.izinler.clone()))
        }
        (Ok(Ok(Some(Kontrol::Devam { .. }))), Giris::Yeni(_)) => {
            reddet(&mut w, c, OTURUM_BITTI).await;
            return Oturum::Reddedildi;
        }
        (Ok(Ok(Some(Kontrol::Merhaba { .. }))), Giris::Devam(_)) => {
            reddet(&mut w, c, KOD_GECERSIZ).await;
            return Oturum::Reddedildi;
        }
        _ => {
            c.close(1u32.into(), b"protokol");
            return Oturum::Reddedildi;
        }
    };
    let ad: String = ad.chars().filter(|c| !c.is_control()).take(40).collect();
    let izinler = match onceki {
        Some(i) => i,
        None => {
            let _ = olay.send(HostOlay::Istek { ad: ad.clone() }).await;
            // Bekleyen eski komutları at.
            while komut.try_recv().is_ok() {}
            let karar = tokio::select! {
                _ = &mut *durdur => return Oturum::Durdur,
                k = tokio::time::timeout(ONAY_SURESI, komut.recv()) => k,
            };
            match karar {
                Ok(Some(HostKomut::Kabul(i))) => i,
                Ok(Some(HostKomut::Red)) | Ok(Some(HostKomut::Kes)) => {
                    reddet(&mut w, c, "Karşı taraf bağlantıyı reddetti.").await;
                    return Oturum::Reddedildi;
                }
                _ => {
                    reddet(&mut w, c, "Karşı taraf zamanında yanıt vermedi.").await;
                    return Oturum::Reddedildi;
                }
            }
        }
    };
    let devam_bilgisi = DevamBilgisi {
        jeton: kod::yeni_bilet(),
        kimlik: c.remote_id(),
        izinler: izinler.clone(),
        ad: ad.clone(),
    };
    // Bağlantı istemeden koptuysa izleyici geri dönebilsin; kapatıldıysa oturum biter.
    let kopus = |sebep: &str| {
        if istemsiz_kopus(c) {
            Oturum::Koptu(devam_bilgisi.clone())
        } else {
            Oturum::Bitti(sebep.to_owned())
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
        let ilk = fab2.yakalayici().and_then(|mut y| {
            let (g, yakalama_ms) = y.yakala_zamanli()?;
            Ok((y, crate::goruntu::yayin_boyutu(g), yakalama_ms))
        });
        let (mut yakalayici, ilk, ilk_yakalama_ms) = match ilk {
            Ok(x) => x,
            Err(e) => {
                let _ = boyut_tx.send(Err(e.to_string()));
                return;
            }
        };
        let _ = boyut_tx.send(Ok((ilk.genislik, ilk.yukseklik)));
        let mut kodlayici = Kodlayici::new(70);
        let mut g = Some((ilk, ilk_yakalama_ms));
        while !dur2.load(std::sync::atomic::Ordering::Relaxed) {
            let t0 = std::time::Instant::now();
            let fps = ayar2.fps.load(std::sync::atomic::Ordering::Relaxed).max(1) as u64;
            let aralik = Duration::from_millis(1000 / fps);
            kodlayici.kalite_ayarla(ayar2.kalite.load(std::sync::atomic::Ordering::Relaxed));
            while let Ok(konumlar) = iptal_rx.try_recv() {
                kodlayici.yeniden_gonder(konumlar);
            }
            let (goruntu, yakalama_ms) = match g.take() {
                Some(x) => x,
                None => match yakalayici.yakala_zamanli() {
                    Ok((x, yakalama_ms)) => (crate::goruntu::yayin_boyutu(x), yakalama_ms),
                    Err(_) => {
                        std::thread::sleep(aralik);
                        continue;
                    }
                },
            };
            if let Ok(Some(mut k)) = kodlayici.kodla(&goruntu) {
                k.yakalama_ms = yakalama_ms;
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
    if protokol::yaz(
        &mut w,
        &Kontrol::Kabul {
            izinler: izinler.clone(),
            genislik,
            yukseklik,
        },
    )
    .await
    .is_err()
        || protokol::yaz(&mut w, &Kontrol::DevamJetonu(devam_bilgisi.jeton.clone()))
            .await
            .is_err()
    {
        yayin_dur.store(true, std::sync::atomic::Ordering::Relaxed);
        return kopus("Bağlantı koptu.");
    }
    let _ = olay
        .send(HostOlay::Baglandi {
            ad,
            izinler: izinler.clone(),
        })
        .await;

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
                    let rtt = ag::rtt(&c2).as_millis().min(u32::MAX as u128) as u32;
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

    // Kontrol akışı ayrı görevde okunur: `oku` iptale dayanıklı değil (yarım okunan mesaj
    // akışı bozar), select! içindeki zamanlayıcılar onu kesmemeli.
    let (gelen_tx, mut gelen_rx) = mpsc::channel::<Kontrol>(16);
    let okuyucu = tokio::spawn(async move {
        while let Ok(Some(m)) = protokol::oku::<_, Kontrol>(&mut r).await {
            if gelen_tx.send(m).await.is_err() {
                break;
            }
        }
    });
    let mut enjektor = if izinler.kontrol {
        fabrika.enjektor().ok()
    } else {
        None
    };
    // Pano yalnız izin verildiyse açılır; izin yoksa hiçbir yönde metin geçmez.
    let mut pano = if izinler.pano {
        fabrika
            .pano()
            .ok()
            .map(|p| tokio::task::block_in_place(|| Esitleyici::new(p)))
    } else {
        None
    };
    let mut pano_saat = tokio::time::interval(PANO_ARALIGI);
    pano_saat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    // Kontrol akışından sonra gelen her çift yönlü akış bir dosya akışıdır. İzin yoksa da
    // kabul edilir: başlık okunur ve izleyiciye nedenini söyleyen `DosyaHata` döner.
    // Ayrı görevde kabul edilir: aşağıdaki select'e dal olarak eklenseydi, bir dosya
    // akışı geldiğinde yarım okunmuş bir kontrol mesajı iptal olup akış kayabilirdi
    // (`protokol::oku` iptale dayanıklı değil).
    let alici = dosya::Alici::new(klasor.to_owned(), izinler.dosya);
    let (c3, olay3) = (c.clone(), olay.clone());
    let dosyalar = tokio::spawn(async move {
        while let Ok((w2, r2)) = c3.accept_bi().await {
            let (alici, olay) = (alici.clone(), olay3.clone());
            tokio::spawn(async move {
                if let Some((ad, yol)) = dosya::al(&alici, w2, r2).await {
                    let _ = olay
                        .send(HostOlay::DosyaAlindi {
                            ad,
                            yol: yol.display().to_string(),
                        })
                        .await;
                }
            });
        }
    });

    let mut sanal_kol = if izinler.oyun_kolu {
        match fabrika.kol_surucusu() {
            Ok(s) => Some(s),
            Err(_) => {
                let _ = olay
                    .send(HostOlay::Uyari(crate::sanal_kol::SURUCU_UYARISI.into()))
                    .await;
                None
            }
        }
    } else {
        None
    };
    let mut kol_siralari = std::collections::HashMap::<u8, u32>::new();
    let mut titresim_araligi = tokio::time::interval(Duration::from_millis(50));
    titresim_araligi.tick().await;
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
            m = gelen_rx.recv() => match m {
                Some(Kontrol::Girdi(g)) => {
                    if let protokol::Girdi::Kol(k) = &g {
                        if sanal_kol.is_none() || !sira_yeni(kol_siralari.get(&k.slot).copied(), k.sira) { continue; }
                        kol_siralari.insert(k.slot, k.sira);
                        let surucu_hatasi = sanal_kol.as_mut().is_some_and(|s| s.guncelle(k).is_err());
                        if surucu_hatasi {
                            sanal_kol = None;
                            let _ = olay.send(HostOlay::Uyari(crate::sanal_kol::SURUCU_UYARISI.into())).await;
                        }
                        continue;
                    }
                    if let Some(e) = enjektor.as_mut() { let _ = tokio::task::block_in_place(|| e.uygula(&g)); }
                }
                Some(Kontrol::SaatSor { izleyici_ms }) => {
                    let host_ms = unix_ms();
                    let _ = protokol::yaz(&mut w, &Kontrol::SaatCevap { izleyici_ms, host_ms }).await;
                }
                Some(Kontrol::Pano(metin)) => {
                    if let Some(p) = pano.as_mut() { let _ = tokio::task::block_in_place(|| p.gelen(&metin)); }
                }
                Some(Kontrol::Kapat(_)) => break Oturum::Bitti("İzleyici bağlantıyı kapattı.".into()),
                None => break kopus("İzleyici bağlantıyı kapattı."),
                Some(_) => {}
            },
            _ = pano_saat.tick(), if pano.is_some() => {
                let yeni = pano.as_mut().and_then(|p| tokio::task::block_in_place(|| p.yokla()));
                if let Some(metin) = yeni {
                    if protokol::yaz(&mut w, &Kontrol::Pano(metin)).await.is_err() {
                        break kopus("Bağlantı koptu.");
                    }
                }
            },
            d = c.read_datagram() => match d {
                Ok(b) => if let Ok(k) = bincode::deserialize::<protokol::KolDurumu>(&b) {
                    if sanal_kol.is_some() && sira_yeni(kol_siralari.get(&k.slot).copied(), k.sira) {
                        kol_siralari.insert(k.slot, k.sira);
                        let surucu_hatasi = sanal_kol.as_mut().is_some_and(|s| s.guncelle(&k).is_err());
                        if surucu_hatasi {
                            sanal_kol = None;
                            let _ = olay.send(HostOlay::Uyari(crate::sanal_kol::SURUCU_UYARISI.into())).await;
                        }
                    }
                },
                Err(_) => {}
            },
            _ = titresim_araligi.tick() => {
                if let Some(s) = sanal_kol.as_mut() {
                    for (slot, buyuk, kucuk) in s.titresim_al() {
                        let _ = protokol::yaz(&mut w, &Kontrol::Titresim { slot, buyuk, kucuk }).await;
                    }
                }
            }
        }
    };
    okuyucu.abort();
    yayin_dur.store(true, std::sync::atomic::Ordering::Relaxed);
    yayin.abort();
    dosyalar.abort();
    if let Some(s) = sanal_kol.as_mut() {
        for slot in kol_siralari.keys().copied().collect::<Vec<_>>() {
            s.kaldir(slot);
        }
    }
    sonuc
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

pub fn sira_yeni(eski: Option<u32>, yeni: u32) -> bool {
    eski.map(|e| yeni != e && yeni.wrapping_sub(e) < (1u32 << 31))
        .unwrap_or(true)
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
