//! Bağlantı veren taraf. iroh uç noktası açar (doğrudan + NAT delme + relay yedeği),
//! kodu üretir, gelen isteği kullanıcı onayına sunar, onaylanırsa ekranı yayınlar ve
//! (izin varsa) girdiyi uygular, panoyu karşı tarafla paylaşır. Bağlantı istemeden
//! koparsa aynı izleyici `DEVAM_SURESI` içinde onay sorulmadan geri dönebilir.
use crate::{
    ag::{self, Baglanti, GonderAkisi, Kurulum},
    dosya,
    goruntu::Kodlayici,
    guvenilen, kimlik,
    kod::{self, Davet},
    pano::{Esitleyici, PANO_ARALIGI},
    platform::Fabrika,
    protokol::{self, Izinler, Kontrol},
};
use anyhow::Result;
use iroh::{Endpoint, EndpointId};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};

/// Kullanıcı bağlantı isteğine bu sürede yanıt vermezse istek reddedilir.
pub const ONAY_SURESI: Duration = Duration::from_secs(60);
/// İstemeden kopan oturuma izleyicinin onaysız geri dönebileceği süre.
pub const DEVAM_SURESI: Duration = Duration::from_secs(120);
/// Kod üretmeden önce relay'e bağlanmak için beklenen en uzun süre.
const CEVRIMICI_BEKLE: Duration = Duration::from_secs(8);
pub const HEDEF_FPS: u64 = 30;
/// Aynı anda yolda olabilecek kare sayısı; aşılırsa en eski kare iptal edilir.
pub const AZAMI_UCUSTA: usize = 3;
pub const AZAMI_PAROLA_DENEMESI: usize = 3;
/// Uygulama kapanış kodu: "ağ değişti, geri döneceğim" — oturum devam bekler.
pub const KOPUS_KODU: u32 = 2;

/// Yayın ayarı: otomatik kalite döngüsü yazar, kodlayıcı iş parçacığı okur.
struct YayinAyari {
    kalite: std::sync::atomic::AtomicU8,
    fps: std::sync::atomic::AtomicU32,
}

/// Oturum erken döndüğünde veya async görev iptal edildiğinde de yakalamayı bitir.
struct YayinKoruma(Arc<AtomicBool>);

impl Drop for YayinKoruma {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

fn terminal_erisimi(kontrol: bool, destek: bool) -> Result<()> {
    anyhow::ensure!(kontrol, "Terminali kullanmak için kontrol izni iste.");
    anyhow::ensure!(
        destek,
        "Terminal bu bilgisayarda kullanılamıyor, masaüstünden devam et."
    );
    Ok(())
}

#[derive(Default)]
struct TerminalYayini {
    oturum: Option<crate::terminal::TerminalOturumu>,
    /// Yalnız son gönderilen kırpılmış karenin koordinatları kullanılır.
    alan: Option<crate::terminal::Alan>,
    ekran: (u32, u32),
    nesil: u64,
}

/// Yakalama iş parçacığı Arc'ı tutsa da oturum biter bitmez geri yükle.
struct TerminalKoruma(Arc<std::sync::Mutex<TerminalYayini>>);
impl Drop for TerminalKoruma {
    fn drop(&mut self) {
        let mut t = self.0.lock().unwrap_or_else(|e| e.into_inner());
        t.oturum.take();
        t.alan = None;
    }
}

/// Ağ durumuna göre kalite/FPS kararı (saf fonksiyon, test edilir).
/// `iptal`: son ölçüm aralığında yetişmediği için iptal edilen kare sayısı.
pub fn uyarla(kalite: u8, fps: u32, rtt_ms: u32, iptal: u32) -> (u8, u32) {
    if iptal > 0 || rtt_ms > 200 {
        (
            kalite.saturating_sub(10).max(35),
            fps.saturating_sub(5).max(10),
        )
    } else if rtt_ms < 80 {
        ((kalite + 5).min(85), (fps + 5).min(60))
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
        /// Kayıtlı (daha önce eşleşmiş) cihaz, kod olmadan bağlanmak istiyor.
        kayitli: bool,
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
    /// Bağlantı istemeden koptu; izleyicinin geri dönmesi bekleniyor (`DEVAM_SURESI`).
    YenidenBekleniyor,
    /// Oturumda veri taşıyan yol değişti: `yol` "doğrudan" | "relay", `adres` karşı uç.
    Yol {
        yol: String,
        adres: String,
    },
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
    /// Boşsa rastgele 10 hane.
    pub parola: String,
    /// Testler için: relay yok, kodda yalnız 127.0.0.1 olsun, dışarı istek çıkmasın.
    pub yalniz_yerel: bool,
    /// Alınan dosyaların klasörü; `None` = İndirilenler\AfuDesk.
    pub dosya_klasoru: Option<PathBuf>,
    /// Kalıcı cihaz kimliği ve kayıtlı cihazlar bu klasörde. `None`: her açılışta yeni
    /// kimlik, cihaz kaydı yok (yalnız kodla bağlanılır).
    pub veri_klasoru: Option<PathBuf>,
    /// Kodla bağlantı kabul edilsin mi. Uygulama arka planda yalnız kayıtlı cihazları
    /// dinlerken `false`; "Bağlantı ver" ekranı açılınca `Host::kod_ac(true)`.
    pub kod_acik: bool,
}

pub struct Host {
    pub olaylar: mpsc::Receiver<HostOlay>,
    komut: mpsc::Sender<HostKomut>,
    durdur: Option<oneshot::Sender<()>>,
    kod_acik: Arc<AtomicBool>,
    /// "Bağlantı ver" kapanınca bekleyen kodu hemen geçersiz kılmak için.
    kod_kapandi: Arc<tokio::sync::Notify>,
    kimlik: EndpointId,
    endpoint: Endpoint,
    #[cfg(windows)]
    mikrofon_koprusu: Option<crate::sanal_mikrofon::MikrofonKoprusu>,
}

impl Host {
    pub fn google_adresleri(&self) -> (Vec<String>, Vec<String>) {
        let adres = self.endpoint.addr();
        (
            adres.ip_addrs().map(ToString::to_string).collect(),
            adres.relay_urls().map(ToString::to_string).collect(),
        )
    }
    pub async fn komut(&self, k: HostKomut) {
        let _ = self.komut.send(k).await;
    }
    /// Olay alıcısından bağımsız komut göndermek için (arayüz köprüsü).
    pub fn komut_gonderici(&self) -> mpsc::Sender<HostKomut> {
        self.komut.clone()
    }
    pub fn durdur(&mut self) {
        #[cfg(windows)]
        if let Some(mut kopru) = self.mikrofon_koprusu.take() {
            kopru.durdur();
        }
        if let Some(d) = self.durdur.take() {
            let _ = d.send(());
        }
    }
    /// Kodla bağlantıyı aç/kapat (kayıtlı cihazlar her durumda onayla bağlanabilir).
    pub fn kod_ac(&self, acik: bool) {
        self.kod_acik.store(acik, Ordering::SeqCst);
        if !acik {
            // Komut kanalı kullanılmaz: açık oturumu ya da bekleyen onayı kesmemeli.
            self.kod_kapandi.notify_one();
        }
    }
    /// Bu host'un cihaz kimliği (kalıcı `veri_klasoru` verildiyse her açılışta aynı).
    pub fn kimlik(&self) -> String {
        ag::kimlik_metni(self.kimlik)
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.durdur();
    }
}

pub async fn baslat(ayar: HostAyar, fabrika: Arc<dyn Fabrika>) -> Result<Host> {
    #[cfg(windows)]
    let geri_yukle = crate::sanal_mikrofon::baslangicta_geri_yukle;
    #[cfg(not(windows))]
    let geri_yukle = || Ok(());
    let mut host = baslat_geri_yuklemeyle(ayar, fabrika, geri_yukle).await?;
    #[cfg(windows)]
    if crate::sanal_mikrofon::cable_var() {
        match crate::sanal_mikrofon::MikrofonKoprusu::baslat() {
            Ok(kopru) => host.mikrofon_koprusu = Some(kopru),
            Err(e) => {
                log::warn!("Mikrofon köprüsü başlatılamadı; host çalışmaya devam ediyor: {e:#}")
            }
        }
    }
    Ok(host)
}

async fn baslat_geri_yuklemeyle(
    ayar: HostAyar,
    fabrika: Arc<dyn Fabrika>,
    geri_yukle: impl FnOnce() -> Result<()> + Send + 'static,
) -> Result<Host> {
    let geri_yukleme = tokio::task::spawn_blocking(geri_yukle)
        .await
        .map_err(anyhow::Error::from)
        .and_then(|sonuc| sonuc);
    let (olay_tx, olaylar) = mpsc::channel(32);
    if let Err(e) = geri_yukleme {
        log::warn!("Önceki mikrofon geri yüklenemedi: {e:#}");
        let _ = olay_tx
            .send(HostOlay::Uyari(
                "Önceki mikrofon geri yüklenemedi, Ses ayarlarından mikrofonunu seç.".into(),
            ))
            .await;
    }
    let (komut, komut_rx) = mpsc::channel(8);
    let (durdur_tx, durdur_rx) = oneshot::channel();
    let kurulum = if ayar.yalniz_yerel {
        Kurulum::YalnizYerel
    } else {
        Kurulum::ortamdan()
    };
    let gizli = match &ayar.veri_klasoru {
        Some(v) => Some(kimlik::yukle_veya_uret(&v.join(kimlik::HOST_DOSYASI))?.gizli),
        None => None,
    };
    let relaylar = ag::relaylar_ortamdan()?;
    let ep = ag::uc_nokta_kimlikli(kurulum, ayar.upnp, gizli, relaylar.clone()).await?;
    let relaylar = relaylar.unwrap_or_default();
    let kod_acik = Arc::new(AtomicBool::new(ayar.kod_acik));
    let kod_kapandi = Arc::new(tokio::sync::Notify::new());
    let kimlik = ep.id();
    let endpoint = ep.clone();
    tokio::spawn(calis(
        ayar,
        fabrika,
        kurulum,
        relaylar,
        ep,
        olay_tx,
        komut_rx,
        durdur_rx,
        kod_acik.clone(),
        kod_kapandi.clone(),
    ));
    Ok(Host {
        olaylar,
        komut,
        durdur: Some(durdur_tx),
        kod_acik,
        kod_kapandi,
        kimlik,
        endpoint,
        #[cfg(windows)]
        mikrofon_koprusu: None,
    })
}

/// Koda yazılacak doğrudan adresler ve relay adresleri.
fn adresleri_topla(
    ep: &Endpoint,
    kurulum: Kurulum,
    relaylar: &[iroh::RelayUrl],
) -> (Vec<String>, Vec<String>) {
    match kurulum {
        Kurulum::YalnizYerel => (
            ep.bound_sockets()
                .iter()
                .filter(|a| a.ip().is_loopback())
                .map(|a| a.to_string())
                .collect(),
            vec![],
        ),
        _ => ag::yayinlanacak(ep, relaylar),
    }
}

pub fn erisim_aciklamasi(internet_yolu_var: bool, yalniz_yerel: bool) -> String {
    if yalniz_yerel {
        "Yalnız bu bilgisayardan.".into()
    } else if internet_yolu_var {
        "İnternetten ulaşılabilir.".into()
    } else {
        "Bu kod yalnız aynı ağda çalışır.".into()
    }
}

fn yalniz_yerel_uyarisi_gerekir(adresler: &[String], relay_ulasildi: bool) -> bool {
    !relay_ulasildi && !genel_adres_var(adresler)
}

fn genel_adres_var(adresler: &[String]) -> bool {
    adresler.iter().any(|adres| {
        adres
            .parse::<std::net::SocketAddr>()
            .ok()
            .is_some_and(|a| ag::genel_mi(a.ip()))
    })
}

fn hatali_bilet_say(denemeler: &mut usize) -> bool {
    *denemeler += 1;
    *denemeler >= AZAMI_PAROLA_DENEMESI
}

#[cfg(test)]
mod guvenlik_testleri {
    use super::*;

    async fn mikrofon_hatasiyla_host_baslat(
        geri_yukle: impl FnOnce() -> Result<()> + Send + 'static,
    ) {
        let mut host = baslat_geri_yuklemeyle(
            HostAyar {
                ad: "Mikrofon kurtarma testi".into(),
                upnp: false,
                parola: "1234567890".into(),
                yalniz_yerel: true,
                dosya_klasoru: None,
                veri_klasoru: None,
                kod_acik: true,
            },
            Arc::new(crate::platform::sahte::SahteFabrika::new(80, 60)),
            geri_yukle,
        )
        .await
        .expect("Mikrofon kurtarma hatası host başlangıcını engellememeli");
        assert_eq!(
            host.olaylar.recv().await,
            Some(HostOlay::Uyari(
                "Önceki mikrofon geri yüklenemedi, Ses ayarlarından mikrofonunu seç.".into()
            ))
        );
        let olay = tokio::time::timeout(Duration::from_secs(10), host.olaylar.recv())
            .await
            .unwrap();
        assert!(matches!(olay, Some(HostOlay::Hazir { .. })), "{olay:?}");
        host.durdur();
    }

    #[tokio::test]
    async fn mikrofon_kurtarma_hatasi_hostu_engellemez() {
        mikrofon_hatasiyla_host_baslat(|| anyhow::bail!("Geçici COM hatası")).await;
    }

    #[tokio::test]
    async fn mikrofon_kurtarma_join_hatasi_hostu_engellemez() {
        mikrofon_hatasiyla_host_baslat(|| panic!("Kurtarma işçisi çöktü")).await;
    }

    struct IzlenenFabrika {
        sahte: crate::platform::sahte::SahteFabrika,
        etkin: Arc<std::sync::atomic::AtomicUsize>,
        birakilan: Arc<std::sync::atomic::AtomicUsize>,
    }

    struct IzlenenYakalayici {
        ic: Box<dyn crate::platform::Yakalayici>,
        etkin: Arc<std::sync::atomic::AtomicUsize>,
        birakilan: Arc<std::sync::atomic::AtomicUsize>,
    }

    impl crate::platform::Yakalayici for IzlenenYakalayici {
        fn yakala(&mut self) -> Result<crate::goruntu::Goruntu> {
            self.ic.yakala()
        }
    }

    impl Drop for IzlenenYakalayici {
        fn drop(&mut self) {
            self.etkin.fetch_sub(1, Ordering::SeqCst);
            self.birakilan.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl Fabrika for IzlenenFabrika {
        fn yakalayici(&self) -> Result<Box<dyn crate::platform::Yakalayici>> {
            let ic = self.sahte.yakalayici()?;
            self.etkin.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(IzlenenYakalayici {
                ic,
                etkin: self.etkin.clone(),
                birakilan: self.birakilan.clone(),
            }))
        }
        fn enjektor(&self) -> Result<Box<dyn crate::platform::Enjektor>> {
            self.sahte.enjektor()
        }
        fn pano(&self) -> Result<Box<dyn crate::pano::Pano>> {
            self.sahte.pano()
        }
        fn kol_surucusu(&self) -> Result<Box<dyn crate::sanal_kol::KolSurucusu>> {
            self.sahte.kol_surucusu()
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn on_oturum_sonrasinda_yakalayici_birikmez() {
        let etkin = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let birakilan = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let fab = Arc::new(IzlenenFabrika {
            sahte: crate::platform::sahte::SahteFabrika::new(80, 60),
            etkin: etkin.clone(),
            birakilan: birakilan.clone(),
        });
        tokio::time::timeout(Duration::from_secs(90), async {
            let mut host = baslat(
                HostAyar {
                    ad: "Kaynak testi".into(),
                    upnp: false,
                    parola: "1234567890".into(),
                    yalniz_yerel: true,
                    dosya_klasoru: None,
                    veri_klasoru: None,
                    kod_acik: true,
                },
                fab,
            )
            .await
            .unwrap();
            let dinleyici = host.endpoint.addr();
            for tur in 1..=3 {
                let kod = loop {
                    if let Some(HostOlay::Hazir { kod, .. }) = host.olaylar.recv().await {
                        break kod;
                    }
                };
                let mut izleyici = crate::izleyici::baglan(&kod, "1234567890", "Sahte telefon")
                    .await
                    .unwrap();
                loop {
                    if let Some(HostOlay::Istek { .. }) = host.olaylar.recv().await {
                        break;
                    }
                }
                host.komut(HostKomut::Kabul(Izinler {
                    kontrol: false,
                    pano: false,
                    dosya: false,
                    oyun_kolu: false,
                }))
                .await;
                loop {
                    if let Some(crate::izleyici::IzleyiciOlay::Kare { .. }) =
                        izleyici.olaylar.recv().await
                    {
                        break;
                    }
                }
                assert_eq!(
                    etkin.load(Ordering::SeqCst),
                    1,
                    "Tur {tur}: tek yakalayıcı olmalı"
                );
                izleyici.kapat();
                while birakilan.load(Ordering::SeqCst) < tur {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                assert_eq!(
                    etkin.load(Ordering::SeqCst),
                    0,
                    "Tur {tur}: yakalayıcı birikti"
                );
                assert_eq!(birakilan.load(Ordering::SeqCst), tur);
                assert_eq!(
                    host.endpoint.addr(),
                    dinleyici,
                    "Tur {tur}: dinleyici değişti"
                );
            }
            host.durdur();
        })
        .await
        .expect("Oturum bittikten sonra yakalayıcı bırakılmadı");
    }

    #[test]
    fn yayin_korumasi_erken_cikista_durdurur() {
        let dur = Arc::new(AtomicBool::new(false));
        {
            let _koruma = YayinKoruma(dur.clone());
            assert!(!dur.load(Ordering::Relaxed));
        }
        assert!(dur.load(Ordering::Relaxed));
    }

    #[test]
    fn terminal_yalniz_kontrol_izni_ve_gercek_fabrika_ile_acilir() {
        assert!(terminal_erisimi(true, true).is_ok());
        assert!(terminal_erisimi(false, true).is_err());
        assert!(terminal_erisimi(true, false).is_err());
        assert!(terminal_erisimi(false, false).is_err());
    }

    #[test]
    fn relay_ve_genel_adres_yoksa_erisim_uyarisi_gerekir() {
        assert!(yalniz_yerel_uyarisi_gerekir(&[], false));
        assert!(!yalniz_yerel_uyarisi_gerekir(&[], true));
        assert!(!yalniz_yerel_uyarisi_gerekir(
            &["8.8.8.8:1234".into()],
            false
        ));
        assert!(yalniz_yerel_uyarisi_gerekir(
            &["192.168.1.3:1234".into()],
            false
        ));
    }

    #[test]
    fn uc_hatali_bilet_kod_dongusunu_yeniler() {
        let mut denemeler = 0;
        assert!(!hatali_bilet_say(&mut denemeler));
        assert!(!hatali_bilet_say(&mut denemeler));
        assert!(hatali_bilet_say(&mut denemeler));
        assert_eq!(denemeler, AZAMI_PAROLA_DENEMESI);
    }

    #[test]
    fn izleyici_yeniden_deneme_jeton_omrunden_kisa() {
        assert!(crate::izleyici::YENIDEN_SURESI <= DEVAM_SURESI);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn kod_ekrani_kapaninca_yeni_kod_uretilir() {
        let fab = Arc::new(crate::platform::sahte::SahteFabrika::new(80, 60));
        let ayar = HostAyar {
            ad: "Deneme".into(),
            upnp: false,
            parola: "1234567890".into(),
            yalniz_yerel: true,
            dosya_klasoru: None,
            veri_klasoru: None,
            kod_acik: true,
        };
        let mut h = baslat(ayar, fab).await.unwrap();
        let eski_kod = loop {
            if let Some(HostOlay::Hazir { kod, .. }) = h.olaylar.recv().await {
                break kod;
            }
        };
        h.kod_ac(false);
        let yeni_kod = tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                if let Some(HostOlay::Hazir { kod, .. }) = h.olaylar.recv().await {
                    break kod;
                }
            }
        })
        .await
        .unwrap();
        assert_ne!(eski_kod, yeni_kod);
        assert!(!h.olaylar.is_closed());
    }
}

/// İzleyicinin onaysız geri dönebilmesi için saklanan oturum bilgisi.
#[derive(Clone)]
struct DevamBilgisi {
    jeton: String,
    kimlik: EndpointId,
    izinler: Izinler,
    ad: String,
    jeton_omru: Duration,
}

/// Kodla ilk kez bağlanan izleyiciye verilecek kayıt bilgisi (veri klasörü varsa).
struct KayitVerisi {
    veri: PathBuf,
    host_ad: String,
    adresler: Vec<String>,
    relaylar: Vec<String>,
}

enum Giris<'a> {
    /// Koddaki tek kullanımlık bilet; kayıtlı cihazlar da bu durumda bağlanabilir.
    Yeni {
        bilet: &'a str,
        kayit: Option<&'a KayitVerisi>,
    },
    Devam(&'a DevamBilgisi),
}

/// Onayı verilen oturumun türü.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tur {
    Kod,
    Kayitli,
    Google,
    Devam,
}

#[allow(clippy::too_many_arguments)]
async fn calis(
    ayar: HostAyar,
    fabrika: Arc<dyn Fabrika>,
    kurulum: Kurulum,
    relaylar: Vec<iroh::RelayUrl>,
    ep: Endpoint,
    olay: mpsc::Sender<HostOlay>,
    mut komut: mpsc::Receiver<HostKomut>,
    mut durdur: oneshot::Receiver<()>,
    kod_acik: Arc<AtomicBool>,
    kod_kapandi: Arc<tokio::sync::Notify>,
) {
    let relay_ulasildi =
        kurulum != Kurulum::YalnizYerel && ag::cevrimici(&ep, CEVRIMICI_BEKLE).await;
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
            let _ = olay.send(HostOlay::YenidenBekleniyor).await;
            let bitis = tokio::time::Instant::now() + d.jeton_omru;
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
                    &ag::kimlik_metni(ep.id()),
                    &baglanti,
                    Giris::Devam(&d),
                    &fabrika,
                    &klasor,
                    &kod_acik,
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
                Some(Oturum::Reddedildi | Oturum::HataliBilet) | None => {
                    let _ = olay
                        .send(HostOlay::Koptu {
                            sebep: "Bağlantı koptu; karşı taraf geri dönmedi.".into(),
                        })
                        .await;
                }
            }
        }

        let (adresler, relaylar_metin) = adresleri_topla(&ep, kurulum, &relaylar);
        if adresler.is_empty() && relaylar_metin.is_empty() {
            let _ = olay
                .send(HostOlay::Hata("Ağ bağlantısı bulunamadı.".into()))
                .await;
            break;
        }
        let bilet = kod::yeni_bilet();
        let kayit = ayar.veri_klasoru.clone().map(|veri| KayitVerisi {
            veri,
            host_ad: ayar.ad.clone(),
            adresler: adresler.clone(),
            relaylar: relaylar_metin.clone(),
        });
        let davet = Davet {
            v: kod::DAVET_SURUMU,
            ad: ayar.ad.clone(),
            adresler: adresler.clone(),
            parmak_izi: ag::kimlik_metni(ep.id()),
            relaylar: relaylar_metin.clone(),
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
                erisim: erisim_aciklamasi(
                    relay_ulasildi || genel_adres_var(&adresler),
                    kurulum == Kurulum::YalnizYerel,
                ),
            })
            .await;
        if kurulum != Kurulum::YalnizYerel
            && yalniz_yerel_uyarisi_gerekir(&adresler, relay_ulasildi)
        {
            let _ = olay
                .send(HostOlay::Uyari("Bu kod yalnız aynı ağda çalışır.".into()))
                .await;
        }
        let bitis = tokio::time::Instant::now() + Duration::from_secs(kod::GECERLILIK_SN as u64);
        let mut hatali_denemeler = 0;
        loop {
            let gelen = tokio::select! {
                _ = &mut durdur => break 'kod,
                _ = tokio::time::sleep_until(bitis) => continue 'kod, // süre doldu: yeni kod
                _ = kod_kapandi.notified() => {
                    if kod_acik.load(Ordering::SeqCst) {
                        continue;
                    }
                    continue 'kod; // eski kod artık kabul edilmez
                }
                g = ep.accept() => g,
            };
            let Some(gelen) = gelen else { break 'kod };
            let Ok(Ok(baglanti)) = tokio::time::timeout(Duration::from_secs(10), gelen).await
            else {
                continue;
            };
            match oturum(
                &ag::kimlik_metni(ep.id()),
                &baglanti,
                Giris::Yeni {
                    bilet: &bilet,
                    kayit: kayit.as_ref(),
                },
                &fabrika,
                &klasor,
                &kod_acik,
                &olay,
                &mut komut,
                &mut durdur,
            )
            .await
            {
                Oturum::HataliBilet => {
                    if hatali_bilet_say(&mut hatali_denemeler) {
                        continue 'kod;
                    }
                    continue;
                }
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
    HataliBilet,
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
    yerel_kimlik: &str,
    c: &Baglanti,
    giris: Giris<'_>,
    fabrika: &Arc<dyn Fabrika>,
    klasor: &Path,
    kod_acik: &AtomicBool,
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
    const OTURUM_BITTI: &str = "Oturum sona ermiş; yeni kod iste.";
    const KOD_GECERSIZ: &str = "Kod geçersiz ya da daha önce kullanılmış.";
    const KOD_KAPALI: &str =
        "Karşı taraf şu an bağlantı vermiyor. AfuDesk'te \"Bağlantı ver\"i açmasını iste.";
    const MESGUL: &str = "Karşı taraf şu an başka bir bağlantıda. Biraz sonra yeniden dene.";
    // (ad, önceden verilmiş izinler, tür): devam eden oturumda onay yeniden sorulmaz.
    let mut google_kanit = None;
    let (ad, onceki, tur) = match (ilk, &giris) {
        (Ok(Ok(Some(Kontrol::Merhaba { bilet: gelen, ad }))), Giris::Yeni { bilet, .. }) => {
            if !kod_acik.load(Ordering::SeqCst) {
                reddet(&mut w, c, KOD_KAPALI).await;
                return Oturum::Reddedildi;
            }
            if !sabit_zamanli_esit(&gelen, bilet) {
                reddet(&mut w, c, KOD_GECERSIZ).await;
                return Oturum::HataliBilet;
            }
            (ad, None, Tur::Kod)
        }
        (Ok(Ok(Some(Kontrol::MerhabaGoogle { kanit, ad }))), _) => {
            let Some(g) = crate::google_baglanti::bul(yerel_kimlik) else {
                reddet(&mut w, c, "Google hesabı kapalı, kodla bağlantıyı kullan.").await;
                return Oturum::Reddedildi;
            };
            let yetki = tokio::time::timeout(
                Duration::from_secs(30),
                g.yetki(&kanit, &ag::kimlik_metni(c.remote_id())),
            )
            .await;
            match yetki {
                Ok(Ok((crate::google::Karar::Kabul, cihaz_adi))) => {
                    google_kanit = Some(kanit);
                    (
                        cihaz_adi,
                        Some(Izinler {
                            kontrol: true,
                            pano: true,
                            dosya: true,
                            oyun_kolu: false,
                        }),
                        Tur::Google,
                    )
                }
                Ok(Ok((crate::google::Karar::OnaySor, _))) => (ad, None, Tur::Google),
                _ => {
                    reddet(
                        &mut w,
                        c,
                        "Google bağlantısı doğrulanamadı, yeniden giriş yap.",
                    )
                    .await;
                    return Oturum::Reddedildi;
                }
            }
        }
        (Ok(Ok(Some(Kontrol::MerhabaKayitli { jeton, ad }))), Giris::Yeni { kayit, .. }) => {
            let Some(k) = kayit else {
                reddet(&mut w, c, guvenilen::GECERSIZ).await;
                return Oturum::Reddedildi;
            };
            // Cihaz kimliği el sıkışmada doğrulandı; jeton bu kimliğe bağlı.
            let izleyici = ag::kimlik_metni(c.remote_id());
            let kayitlar = guvenilen::oku(&guvenilen::kayit_yolu(&k.veri));
            match kayitlar
                .hostlar
                .iter()
                .find(|h| h.izleyici_kimlik == izleyici)
            {
                None => {
                    reddet(&mut w, c, guvenilen::KALDIRILDI).await;
                    return Oturum::Reddedildi;
                }
                Some(h) if !guvenilen::jeton_dogrula(&jeton, &h.jeton_sha256) => {
                    reddet(&mut w, c, guvenilen::GECERSIZ).await;
                    return Oturum::Reddedildi;
                }
                Some(_) => (ad, None, Tur::Kayitli),
            }
        }
        (Ok(Ok(Some(Kontrol::Devam { jeton, .. }))), Giris::Devam(d)) => {
            // Jeton + cihaz kimliği birlikte: jetonu ele geçiren başka cihaz dönemez.
            if !sabit_zamanli_esit(&jeton, &d.jeton) || c.remote_id() != d.kimlik {
                reddet(&mut w, c, OTURUM_BITTI).await;
                return Oturum::Reddedildi;
            }
            (d.ad.clone(), Some(d.izinler.clone()), Tur::Devam)
        }
        (Ok(Ok(Some(Kontrol::Devam { .. }))), Giris::Yeni { .. }) => {
            reddet(&mut w, c, OTURUM_BITTI).await;
            return Oturum::Reddedildi;
        }
        (Ok(Ok(Some(Kontrol::MerhabaKayitli { .. }))), Giris::Devam(_)) => {
            reddet(&mut w, c, MESGUL).await;
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
            // Bekleyen eski komutları İSTEKTEN ÖNCE at: sonra atılsaydı, isteği görüp hemen
            // verilen onay da silinebilirdi (yarış).
            while komut.try_recv().is_ok() {}
            let _ = olay
                .send(HostOlay::Istek {
                    ad: ad.clone(),
                    kayitli: tur == Tur::Kayitli,
                })
                .await;
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
        jeton_omru: DEVAM_SURESI,
    };
    // Bağlantı istemeden koptuysa izleyici geri dönebilsin; kapatıldıysa oturum biter.
    let kopus = |sebep: &str| {
        if google_kanit.is_none() && istemsiz_kopus(c) {
            Oturum::Koptu(devam_bilgisi.clone())
        } else {
            Oturum::Bitti(sebep.to_owned())
        }
    };
    // Görüntü: yakalayıcı kendi iş parçacığında oluşturulur (platform tutamaçları Send değil).
    let terminal = Arc::new(std::sync::Mutex::new(TerminalYayini::default()));
    let _terminal_koruma = TerminalKoruma(terminal.clone());
    let terminal2 = terminal.clone();
    let (terminal_hata_tx, mut terminal_hata_rx) = mpsc::channel::<u64>(1);
    let (nesil_tx, mut nesil_rx) = tokio::sync::watch::channel(0u64);
    let (kare_tx, mut kare_rx) = mpsc::channel(2);
    let ayar = Arc::new(YayinAyari {
        kalite: std::sync::atomic::AtomicU8::new(70),
        fps: std::sync::atomic::AtomicU32::new(HEDEF_FPS as u32),
    });
    let ayar2 = ayar.clone();
    let (iptal_tx, iptal_rx) = std::sync::mpsc::channel::<Vec<(u32, u32)>>();
    let (boyut_tx, boyut_rx) = oneshot::channel::<Result<(u32, u32), String>>();
    let yayin_dur = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let _yayin_koruma = YayinKoruma(yayin_dur.clone());
    let dur2 = yayin_dur.clone();
    let fab2 = fabrika.clone();
    std::thread::spawn(move || {
        let ilk = fab2.yakalayici().and_then(|mut y| {
            let (g, yakalama_ms) = y.yakala_zamanli()?;
            Ok((y, g, yakalama_ms))
        });
        let (mut yakalayici, ilk, ilk_yakalama_ms) = match ilk {
            Ok(x) => x,
            Err(e) => {
                let _ = boyut_tx.send(Err(e.to_string()));
                return;
            }
        };
        terminal2.lock().unwrap().ekran = (ilk.genislik, ilk.yukseklik);
        let ilk_boyut = crate::goruntu::yayin_boyutu(ilk.clone());
        let _ = boyut_tx.send(Ok((ilk_boyut.genislik, ilk_boyut.yukseklik)));
        let mut kodlayici = Kodlayici::new(70);
        let mut son_nesil = 0;
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
                    Ok((x, yakalama_ms)) => (x, yakalama_ms),
                    Err(_) => {
                        std::thread::sleep(aralik);
                        continue;
                    }
                },
            };
            let hazir = {
                let mut t = terminal2.lock().unwrap();
                t.ekran = (goruntu.genislik, goruntu.yukseklik);
                let nesil = t.nesil;
                if let Some(o) = &t.oturum {
                    match o.alan().and_then(|a| {
                        let a = crate::terminal::kesisim(a, goruntu.genislik, goruntu.yukseklik)?;
                        crate::terminal::kirp(&goruntu, a).map(|g| (g, a))
                    }) {
                        Ok((g, a)) => {
                            t.alan = Some(a);
                            Some((g, nesil))
                        }
                        Err(_) => {
                            // Hata anında masaüstünü terminal karesi diye yayınlama.
                            t.alan = None;
                            let _ = terminal_hata_tx.try_send(nesil);
                            None
                        }
                    }
                } else {
                    t.alan = None;
                    Some((goruntu, nesil))
                }
            };
            let Some((goruntu, nesil)) = hazir else {
                std::thread::sleep(aralik);
                continue;
            };
            if nesil != son_nesil {
                kodlayici.tam_kare_iste();
                son_nesil = nesil;
            }
            let goruntu = crate::goruntu::yayin_boyutu(goruntu);
            if let Ok(Some(mut k)) = kodlayici.kodla(&goruntu) {
                k.yakalama_ms = yakalama_ms;
                if kare_tx.blocking_send((k, nesil)).is_err() {
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
    // Jeton Kabul'den ÖNCE gider: izleyici Kabul'ü gördüğünde geri dönüş jetonu hazırdır
    // (hemen ardından kopan bağlantı da devam edebilir).
    if protokol::yaz(&mut w, &Kontrol::DevamJetonu(devam_bilgisi.jeton.clone()))
        .await
        .is_err()
        || protokol::yaz(
            &mut w,
            &Kontrol::Kabul {
                izinler: izinler.clone(),
                genislik,
                yukseklik,
            },
        )
        .await
        .is_err()
    {
        yayin_dur.store(true, std::sync::atomic::Ordering::Relaxed);
        return kopus("Bağlantı koptu.");
    }
    if let Giris::Yeni { kayit: Some(k), .. } = &giris {
        let yol = guvenilen::kayit_yolu(&k.veri);
        let izleyici = ag::kimlik_metni(c.remote_id());
        match tur {
            Tur::Kod => {
                // Cihazlar birbirini hatırlasın: bundan sonra kod gerekmez (onay yine sorulur).
                let jeton = guvenilen::yeni_jeton();
                let simdi = guvenilen::simdi();
                let kayit = guvenilen::HostKaydi {
                    etiket: None,
                    izleyici_kimlik: izleyici,
                    ad: ad.clone(),
                    jeton_sha256: guvenilen::jeton_ozeti(&jeton),
                    eklenme: simdi,
                    son_gorulme: simdi,
                };
                if guvenilen::guncelle(&yol, |kk| guvenilen::hosta_ekle(kk, kayit)).is_ok() {
                    let _ = protokol::yaz(
                        &mut w,
                        &Kontrol::KayitJetonu {
                            host_ad: k.host_ad.clone(),
                            adresler: k.adresler.clone(),
                            relaylar: k.relaylar.clone(),
                            jeton,
                        },
                    )
                    .await;
                }
            }
            Tur::Kayitli => {
                let _ = guvenilen::guncelle(&yol, |kk| {
                    if let Some(h) = kk
                        .hostlar
                        .iter_mut()
                        .find(|h| h.izleyici_kimlik == izleyici)
                    {
                        h.son_gorulme = guvenilen::simdi();
                    }
                });
            }
            Tur::Devam | Tur::Google => {}
        }
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
            nesil: u64,
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
                    let Some((k, nesil)) = k else { break };
                    if nesil != *nesil_rx.borrow() { continue; }
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
                    ucusta.push_back(Ucusta { nesil, konumlar, iptal: Some(iptal), gorev });
                }
                degisti = nesil_rx.changed() => {
                    if degisti.is_err() { break; }
                    let simdiki = *nesil_rx.borrow_and_update();
                    ucusta.retain_mut(|eski| {
                        if eski.nesil == simdiki { return true; }
                        if let Some(i) = eski.iptal.take() { let _ = i.send(()); }
                        eski.gorev.abort();
                        false
                    });
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
    let (gorsel_tx, mut gorsel_rx) = mpsc::channel::<PathBuf>(16);
    let (c3, olay3) = (c.clone(), olay.clone());
    let dosyalar = tokio::spawn(async move {
        while let Ok((w2, r2)) = c3.accept_bi().await {
            let (alici, olay) = (alici.clone(), olay3.clone());
            let gorsel_tx = gorsel_tx.clone();
            tokio::spawn(async move {
                if let Some((ad, yol)) = dosya::al(&alici, w2, r2).await {
                    // Aynı pano nesnesi oturum boyunca yaşar; pano izni ayrıca korunur.
                    let _ = gorsel_tx.send(yol.clone()).await;
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
    // Yakalama/odak hatası terminal girdisini masaüstüne düşürmemeli.
    // Kilit yalnız açık bir TerminalModu { ac: None } komutuyla kalkar.
    let mut terminal_secili = false;
    let mut terminal_uyari_zamani: Option<std::time::Instant> = None;
    let mut bekleyen_birakmalar = Vec::new();
    let mut telmik = crate::telmik_sayac::Sayac::yeni("host", "alinan");
    let mut titresim_araligi = tokio::time::interval(Duration::from_millis(50));
    titresim_araligi.tick().await;
    let mut yol_olcum = tokio::time::interval(Duration::from_secs(1));
    let mut son_yol = (String::new(), String::new());
    let mut google_denetim = tokio::time::interval(Duration::from_secs(30));
    let sonuc = loop {
        tokio::select! {
            _ = &mut *durdur => { c.close(0u32.into(), b"durdu"); break Oturum::Durdur; }
            _ = google_denetim.tick(), if google_kanit.is_some() => {
                let sonuc = match crate::google_baglanti::bul(yerel_kimlik) {
                    Some(g) => tokio::time::timeout(Duration::from_secs(30),
                        g.yetki(google_kanit.as_ref().expect("google proof"), &ag::kimlik_metni(c.remote_id()))).await,
                    None => { c.close(0u32.into(),b"google signout"); break Oturum::Bitti("Hesaptan çıkıldı.".into()); }
                };
                if !matches!(sonuc,Ok(Ok((crate::google::Karar::Kabul,_)))) {
                    c.close(0u32.into(),b"google revoked"); break Oturum::Bitti("Cihaz erişimi sona erdi, yeniden giriş yap.".into());
                }
            },
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
                Some(Kontrol::TerminalListesi) => {
                    let sonuc = terminal_erisimi(izinler.kontrol, fabrika.terminal_destegi())
                        .and_then(|_| crate::terminal::listele());
                    let mesaj = match sonuc {
                        Ok(p) => Kontrol::TerminalPencereleri(p),
                        Err(e) => Kontrol::TerminalDurum { acik: false, hata: Some(e.to_string()) },
                    };
                    if protokol::yaz(&mut w, &mesaj).await.is_err() { break kopus("Bağlantı koptu."); }
                }
                Some(Kontrol::TerminalModu { ac, en, boy }) => {
                    terminal_secili = ac.is_some();
                    terminal_uyari_zamani = None;
                    let sonuc = if let Some(id) = ac {
                        terminal_erisimi(izinler.kontrol, fabrika.terminal_destegi()).and_then(|_| {
                            let mut t = terminal.lock().unwrap();
                            // Önceki pencereyi geri yüklemeden yeni guard oluşturma.
                            t.oturum.take();
                            t.alan = None;
                            t.nesil += 1;
                            nesil_tx.send_replace(t.nesil);
                            crate::terminal::TerminalOturumu::ac(id, en, boy).map(|o| t.oturum = Some(o))
                        })
                    } else {
                        let mut t = terminal.lock().unwrap();
                        t.oturum.take();
                        t.alan = None;
                        t.nesil += 1;
                        nesil_tx.send_replace(t.nesil);
                        Ok(())
                    };
                    let acik = terminal.lock().unwrap().oturum.is_some();
                    let mesaj = Kontrol::TerminalDurum { acik, hata: sonuc.err().map(|e| e.to_string()) };
                    if protokol::yaz(&mut w, &mesaj).await.is_err() { break kopus("Bağlantı koptu."); }
                }
                Some(Kontrol::Girdi(mut g)) => {
                    // Ses odak/terminal dönüşümünden bağımsızdır; kontrol izni gerekir.
                    if protokol::ses_girdisi(&g) {
                        if !izinler.kontrol { continue; }
                        telmik.isle(&g);
                        if let Some(e) = enjektor.as_mut() {
                            if let Err(hata) = tokio::task::block_in_place(|| e.uygula(&g)) {
                                let mesaj = crate::sanal_mikrofon::kullanici_uyarisi(&hata);
                                let _ = olay.send(HostOlay::Uyari(mesaj.into())).await;
                            }
                        }
                        continue;
                    }
                    if let protokol::Girdi::Kol(k) = &g {
                        if terminal_secili { continue; }
                        if sanal_kol.is_none() || !sira_yeni(kol_siralari.get(&k.slot).copied(), k.sira) { continue; }
                        kol_siralari.insert(k.slot, k.sira);
                        let surucu_hatasi = sanal_kol.as_mut().is_some_and(|s| s.guncelle(k).is_err());
                        if surucu_hatasi {
                            sanal_kol = None;
                            let _ = olay.send(HostOlay::Uyari(crate::sanal_kol::SURUCU_UYARISI.into())).await;
                        }
                        continue;
                    }
                    let sonuc = {
                        let t = terminal.lock().unwrap();
                        tokio::task::block_in_place(|| crate::terminal::korumali_girdi(
                            terminal_secili,
                            || match &t.oturum {
                                Some(o) => o.odakla(),
                                None => anyhow::bail!("Terminal kapanmış, yeniden seç."),
                            },
                            || {
                                if terminal_secili {
                                    if let protokol::Girdi::FareKonum { x, y } = &mut g {
                                        let a = t.alan.ok_or_else(|| anyhow::anyhow!("Terminal hazır değil, yeniden dene."))?;
                                        (*x, *y) = crate::terminal::fare_cevir(*x, *y, a, t.ekran.0, t.ekran.1);
                                    }
                                }
                                if let Some(e) = enjektor.as_mut() {
                                    for birak in bekleyen_birakmalar.drain(..) { e.uygula(&birak)?; }
                                    e.uygula(&g)?;
                                }
                                Ok(())
                            },
                        ))
                    };
                    if sonuc.is_err() && terminal_secili {
                        if matches!(&g, protokol::Girdi::Tus { basili: false, .. } |
                            protokol::Girdi::FareTus { basili: false, .. })
                            && !bekleyen_birakmalar.contains(&g) {
                            bekleyen_birakmalar.push(g);
                        }
                        if terminal_uyari_zamani.is_none_or(|z| z.elapsed() >= Duration::from_secs(2)) {
                            terminal_uyari_zamani = Some(std::time::Instant::now());
                            let mesaj = Kontrol::TerminalDurum { acik: true,
                                hata: Some("Terminale yazılamadı, bilgisayarda terminali seçip yeniden dene.".into()) };
                            if protokol::yaz(&mut w, &mesaj).await.is_err() { break kopus("Bağlantı koptu."); }
                        }
                    }
                }
                Some(Kontrol::SaatSor { izleyici_ms }) => {
                    // Ses yanıtı da eski istemcilerin okuyabildiği SaatCevap biçimindedir.
                    if let Some(yanit) = protokol::yetenek_yaniti(izleyici_ms) {
                        if protokol::yaz(&mut w, &yanit).await.is_err() { break kopus("Bağlantı koptu."); }
                        continue;
                    }
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
            Some(hatali_nesil) = terminal_hata_rx.recv() => {
                {
                    let mut t = terminal.lock().unwrap();
                    if t.nesil != hatali_nesil || t.oturum.is_none() { continue; }
                    t.oturum.take();
                    t.alan = None;
                    t.nesil += 1;
                    nesil_tx.send_replace(t.nesil);
                }
                let mesaj = Kontrol::TerminalDurum {
                    acik: false, hata: Some("Terminal görüntülenemedi, yeniden seç.".into()),
                };
                if protokol::yaz(&mut w, &mesaj).await.is_err() { break kopus("Bağlantı koptu."); }
            }
            Some(yol) = gorsel_rx.recv() => {
                if let Some(p) = pano.as_mut() {
                    if let Err(hata) = tokio::task::block_in_place(|| p.gelen_gorsel(&yol)) {
                        log::warn!("Alınan görsel panoya konamadı: {hata}");
                        let _ = olay.send(HostOlay::Uyari("Görsel kaydedildi ama panoya konamadı; AfuDesk Gelen klasöründen aç.".into())).await;
                    }
                }
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
            _ = yol_olcum.tick() => {
                let yeni = (ag::secili_yol(c).0.ad().to_owned(), ag::secili_yol_adresi(c));
                if !yeni.1.is_empty() && yeni != son_yol {
                    son_yol = yeni.clone();
                    let _ = olay.send(HostOlay::Yol { yol: yeni.0, adres: yeni.1 }).await;
                }
            }
            _ = titresim_araligi.tick() => {
                if let Some(s) = sanal_kol.as_mut() {
                    for (slot, buyuk, kucuk) in s.titresim_al() {
                        let _ = protokol::yaz(&mut w, &Kontrol::Titresim { slot, buyuk, kucuk }).await;
                    }
                }
            }
        }
    };
    #[cfg(test)]
    eprintln!("[host] oturum bitti: kapanis={:?}", c.close_reason());
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
        assert_eq!(uyarla(70, 30, 250, 0), (60, 25));
        assert_eq!(uyarla(70, 30, 30, 2), (60, 25), "iptal varsa düşer");
        assert_eq!(uyarla(36, 12, 300, 5), (35, 10), "alt sınır");
    }

    #[test]
    fn iyi_agda_yukselir_ust_sinirda_durur() {
        assert_eq!(uyarla(70, 30, 20, 0), (75, 35));
        assert_eq!(uyarla(80, 58, 20, 0), (85, 60));
        assert_eq!(uyarla(85, 60, 20, 0), (85, 60));
    }

    #[test]
    fn orta_agda_sabit() {
        assert_eq!(uyarla(60, 30, 120, 0), (60, 30));
    }
}
