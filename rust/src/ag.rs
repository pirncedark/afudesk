//! Taşıma katmanı: iroh (QUIC). Her cihaz bir anahtar çiftiyle tanınır; açık anahtar
//! (uç kimliği) koddan gelir ve el sıkışmada doğrulanır — başka cihaz araya giremez.
//!
//! Bağlantı sırası kullanıcıdan hiçbir ayar istemez:
//! 1. Koddaki doğrudan adresler (LAN, IPv6, UPnP/PCP ile açılan dış adres) denenir.
//! 2. İki taraf relay üzerinden buluşur (rendezvous) ve NAT delme (hole punching) yapılır.
//! 3. Doğrudan yol kurulamazsa trafik relay üzerinden akar; sonradan doğrudan yol açılırsa
//!    bağlantı kopmadan oraya geçer. Relay yalnız şifreli paket taşır, içeriği göremez.
use crate::protokol::ALPN;
use anyhow::{Context, Result};
use iroh::{
    endpoint::{presets, QuicTransportConfig},
    Endpoint, EndpointAddr, EndpointId, RelayMode, RelayUrl, TransportAddr,
};
use std::{net::SocketAddr, time::Duration};

pub use iroh::endpoint::{Connection as Baglanti, RecvStream as AlAkisi, SendStream as GonderAkisi};

/// Doğrudan yolu bilerek kapatır (yalnız relay): TEST-2 ve sorun ayıklama için.
pub const SADECE_RELAY_ORTAM: &str = "AFUDESK_SADECE_RELAY";
/// UDP soketini belirli yerel IP'ye bağlar (ör. telefon paylaşımı arayüzü): trafik o
/// arayüzden çıkar. Tek bilgisayarda iki farklı ISS ile test için.
pub const BAGLA_IP_ORTAM: &str = "AFUDESK_BAGLA_IP";
/// Relay (HTTPS) bağlantısı için HTTP CONNECT vekili, ör. `http://127.0.0.1:18080`.
pub const PROXY_ORTAM: &str = "AFUDESK_PROXY";
/// (Yalnız `wan_testi` özelliğiyle) yerel/özel adresli yollar hiç seçilmez: aynı
/// bilgisayardaki test, LAN kısayolundan değil gerçek internetten geçer.
pub const GENEL_YOL_ORTAM: &str = "AFUDESK_TEST_GENEL_YOL";

/// Uç noktanın nasıl kurulacağı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kurulum {
    /// Gerçek kullanım: doğrudan + NAT delme + relay yedeği.
    Internet,
    /// Gerçek kullanım ama doğrudan UDP kapalı: her şey relay'den geçer.
    SadeceRelay,
    /// Testler: relay yok, dışarı hiçbir istek çıkmaz, yalnız 127.0.0.1.
    YalnizYerel,
}

impl Kurulum {
    /// `AFUDESK_SADECE_RELAY=1` ise relay'e zorlanır.
    pub fn ortamdan() -> Self {
        Self::degerden(std::env::var(SADECE_RELAY_ORTAM).ok().as_deref())
    }

    fn degerden(v: Option<&str>) -> Self {
        match v.map(str::trim) {
            Some("1") => Kurulum::SadeceRelay,
            _ => Kurulum::Internet,
        }
    }
}

fn tasima() -> QuicTransportConfig {
    QuicTransportConfig::builder()
        .datagram_receive_buffer_size(Some(64 * 1024))
        .keep_alive_interval(Duration::from_secs(5))
        .max_idle_timeout(Some(Duration::from_secs(20).try_into().expect("süre")))
        .build()
}

pub async fn uc_nokta(kurulum: Kurulum, portmapper: bool) -> Result<Endpoint> {
    uc_nokta_kimlikli(kurulum, portmapper, None).await
}

/// mDNS servis adı: `_afudesk._udp.local`. Aynı ağdaki kayıtlı cihaz, IP'si değişse de
/// internet olmadan bulunur.
pub const MDNS_SERVISI: &str = "afudesk";

/// `gizli`: kalıcı cihaz anahtarı (kayıtlı cihazlar); `None` = her açılışta yeni kimlik.
pub async fn uc_nokta_kimlikli(
    kurulum: Kurulum,
    portmapper: bool,
    gizli: Option<iroh::SecretKey>,
) -> Result<Endpoint> {
    let b = match kurulum {
        Kurulum::YalnizYerel => Endpoint::builder(presets::Minimal)
            .relay_mode(RelayMode::Disabled)
            .clear_ip_transports()
            .bind_addr("127.0.0.1:0")?,
        Kurulum::Internet => Endpoint::builder(presets::N0),
        Kurulum::SadeceRelay => Endpoint::builder(presets::N0).clear_ip_transports(),
    };
    let mut b = b.alpns(vec![ALPN.to_vec()]).transport_config(tasima());
    if let Some(g) = gizli {
        b = b.secret_key(g);
    }
    if kurulum == Kurulum::Internet {
        b = b.address_lookup(
            iroh_mdns_address_lookup::MdnsAddressLookup::builder().service_name(MDNS_SERVISI),
        );
    }
    if !portmapper || kurulum != Kurulum::Internet {
        b = b.portmapper_config(iroh::endpoint::PortmapperConfig::Disabled);
    }
    if kurulum != Kurulum::YalnizYerel {
        if kurulum == Kurulum::Internet {
            if let Some(ip) = ortam(BAGLA_IP_ORTAM).and_then(|v| v.parse::<std::net::IpAddr>().ok()) {
                b = b.clear_ip_transports().bind_addr(SocketAddr::new(ip, 0))?;
            }
        }
        if let Some(url) = ortam(PROXY_ORTAM).and_then(|v| v.parse().ok()) {
            b = b.proxy_url(url);
        }
        #[cfg(feature = "wan_testi")]
        if ortam(GENEL_YOL_ORTAM).as_deref() == Some("1") {
            b = b.path_selector(std::sync::Arc::new(test_secici::GenelYolSecici));
        }
    }
    b.bind().await.context("Ağ başlatılamadı.")
}

fn ortam(ad: &str) -> Option<String> {
    std::env::var(ad).ok().map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
}

/// Adres genel internette mi (özel ağ, döngü, bağlantı-yerel, CGNAT değil)?
pub fn genel_mi(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(a) => {
            let o = a.octets();
            !(a.is_private()
                || a.is_loopback()
                || a.is_link_local()
                || a.is_unspecified()
                || a.is_broadcast()
                || (o[0] == 100 && (o[1] & 0xc0) == 64))
        }
        std::net::IpAddr::V6(a) => a.segments()[0] & 0xe000 == 0x2000,
    }
}

#[cfg(feature = "wan_testi")]
pub mod test_secici {
    //! Yalnız genel adresli doğrudan yolları ya da relay'i seçer (doğrudan önce).
    use iroh::endpoint::transports::{
        Addr, PathSelection, PathSelectionContext, PathSelectionData, PathSelector,
    };
    use std::time::Duration;

    #[derive(Debug, Default)]
    pub struct GenelYolSecici;

    impl PathSelector for GenelYolSecici {
        fn select(&self, ctx: &PathSelectionContext<'_>) -> PathSelection {
            let mut en_iyi: Option<(PathSelectionData<'_>, (u8, Duration))> = None;
            for p in ctx.paths() {
                let Some(st) = p.stats() else { continue };
                let kat = match p.network_path().remote() {
                    Addr::Ip(s) if super::genel_mi(s.ip()) => 0,
                    Addr::Relay(..) => 1,
                    _ => continue,
                };
                let anahtar = (kat, st.rtt);
                if en_iyi.as_ref().is_none_or(|(_, a)| anahtar < *a) {
                    en_iyi = Some((p, anahtar));
                }
            }
            let mut secim = PathSelection::none();
            if let Some((p, _)) = en_iyi {
                if ctx.current() != Some(p.network_path()) {
                    secim.set(&p);
                }
            }
            secim
        }
    }
}

/// Relay'e bağlanana kadar (en çok `sure`) bekler. Relay yoksa (ör. internet yok) false.
pub async fn cevrimici(ep: &Endpoint, sure: Duration) -> bool {
    tokio::time::timeout(sure, ep.online()).await.is_ok()
}

/// Uç kimliği metni (koda yazılan): 64 hex.
pub fn kimlik_metni(id: EndpointId) -> String {
    id.to_string()
}

/// Kodun taşıdığı parçalardan bağlanılacak adres.
pub fn hedef_adres(kimlik: &str, adresler: &[String], relaylar: &[String]) -> Result<EndpointAddr> {
    let id: EndpointId = kimlik
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("Kod bozuk: cihaz kimliği okunamadı."))?;
    let ipler = adresler
        .iter()
        .filter_map(|a| a.parse::<SocketAddr>().ok())
        .map(TransportAddr::Ip);
    let relay = relaylar
        .iter()
        .filter_map(|r| r.parse::<RelayUrl>().ok())
        .map(TransportAddr::Relay);
    Ok(EndpointAddr::from_parts(id, ipler.chain(relay)))
}

/// Koda yazılacak adres listesi ve relay adresleri.
pub fn yayinlanacak(ep: &Endpoint) -> (Vec<String>, Vec<String>) {
    let a = ep.addr();
    let mut ipler: Vec<String> = a.ip_addrs().map(|s| s.to_string()).collect();
    ipler.sort();
    ipler.dedup();
    let relaylar = a.relay_urls().map(|r| r.to_string()).collect();
    (ipler, relaylar)
}

/// Kullanıcıya gösterilen bağlantı hatası: teknik ayrıntı (adres, iç hata) içermez;
/// ayrıntı yalnız günlüğe yazılır.
pub const ULASILAMADI: &str =
    "Karşı bilgisayara ulaşılamadı.\nBağlantı veren bilgisayar açık ve internete bağlı mı?";
pub const GUVENLIK_HATASI: &str =
    "Güvenlik kontrolü başarısız; doğru cihazın bağlantı kodunu kullanın.";

pub async fn baglan(ep: &Endpoint, hedef: EndpointAddr, sure: Duration) -> Result<Baglanti> {
    let beklenen = hedef.id;
    let c = match tokio::time::timeout(sure, ep.connect(hedef, ALPN)).await {
        Err(_) => {
            log::warn!("bağlantı zaman aşımı ({sure:?})");
            anyhow::bail!(ULASILAMADI)
        }
        Ok(Err(e)) => {
            log::warn!("bağlantı kurulamadı: {e:#}");
            anyhow::bail!(ULASILAMADI)
        }
        Ok(Ok(c)) => c,
    };
    // iroh el sıkışmada zaten doğrular; yine de açıkça kontrol et.
    anyhow::ensure!(c.remote_id() == beklenen, GUVENLIK_HATASI);
    Ok(c)
}

/// Şu an veri taşıyan yol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Yol {
    Dogrudan,
    Relay,
    Bilinmiyor,
}

impl Yol {
    pub fn ad(self) -> &'static str {
        match self {
            Yol::Dogrudan => "doğrudan",
            Yol::Relay => "relay",
            Yol::Bilinmiyor => "?",
        }
    }
}

pub fn secili_yol(c: &Baglanti) -> (Yol, Duration) {
    for p in c.paths().iter() {
        if p.is_selected() {
            let y = if p.is_relay() { Yol::Relay } else { Yol::Dogrudan };
            return (y, p.rtt());
        }
    }
    (Yol::Bilinmiyor, Duration::ZERO)
}

pub fn rtt(c: &Baglanti) -> Duration {
    secili_yol(c).1
}

/// Seçili yolun karşı ucu: IP:port ya da relay adresi (kanıt/sorun ayıklama için).
pub fn secili_yol_adresi(c: &Baglanti) -> String {
    c.paths()
        .iter()
        .find(|p| p.is_selected())
        .map(|p| p.remote_addr().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod testler {
    use super::*;

    async fn yankici() -> (Endpoint, String, Vec<String>) {
        let ep = uc_nokta(Kurulum::YalnizYerel, false).await.unwrap();
        let e = ep.clone();
        tokio::spawn(async move {
            while let Some(g) = e.accept().await {
                tokio::spawn(async move {
                    let c = g.await?;
                    let (mut w, mut r) = c.accept_bi().await?;
                    let v = r.read_to_end(1024).await?;
                    w.write_all(&v).await?;
                    w.finish()?;
                    c.closed().await;
                    Ok::<_, anyhow::Error>(())
                });
            }
        });
        let adres: Vec<String> = ep.bound_sockets().iter().map(|s| s.to_string()).collect();
        (ep.clone(), kimlik_metni(ep.id()), adres)
    }

    #[tokio::test]
    async fn dogru_kimlige_baglanir_ve_yol_dogrudan() {
        let (_h, kimlik, adres) = yankici().await;
        let ep = uc_nokta(Kurulum::YalnizYerel, false).await.unwrap();
        let hedef = hedef_adres(&kimlik, &adres, &[]).unwrap();
        let c = baglan(&ep, hedef, Duration::from_secs(5)).await.unwrap();
        let (mut w, mut r) = c.open_bi().await.unwrap();
        w.write_all(b"merhaba").await.unwrap();
        w.finish().unwrap();
        assert_eq!(r.read_to_end(1024).await.unwrap(), b"merhaba");
        assert_eq!(secili_yol(&c).0, Yol::Dogrudan);
    }

    #[tokio::test]
    async fn yanlis_kimlik_reddedilir() {
        let (_h, _kimlik, adres) = yankici().await;
        let sahte = uc_nokta(Kurulum::YalnizYerel, false).await.unwrap();
        let ep = uc_nokta(Kurulum::YalnizYerel, false).await.unwrap();
        let hedef = hedef_adres(&kimlik_metni(sahte.id()), &adres, &[]).unwrap();
        assert!(baglan(&ep, hedef, Duration::from_secs(3)).await.is_err());
    }

    #[tokio::test]
    async fn olu_ve_gecersiz_adresler_digerini_engellemez() {
        let (_h, kimlik, mut adres) = yankici().await;
        adres.insert(0, "127.0.0.1:9".into());
        adres.insert(0, "gecersiz".into());
        let ep = uc_nokta(Kurulum::YalnizYerel, false).await.unwrap();
        let hedef = hedef_adres(&kimlik, &adres, &[]).unwrap();
        assert!(baglan(&ep, hedef, Duration::from_secs(5)).await.is_ok());
    }

    #[tokio::test]
    async fn hic_ulasilamazsa_anlasilir_hata() {
        let (h, kimlik, _) = yankici().await;
        drop(h);
        let ep = uc_nokta(Kurulum::YalnizYerel, false).await.unwrap();
        let hedef = hedef_adres(&kimlik, &["127.0.0.1:9".into()], &[]).unwrap();
        let e = baglan(&ep, hedef, Duration::from_secs(2)).await.unwrap_err();
        assert_eq!(e.to_string(), ULASILAMADI);
        assert!(hedef_adres("bozuk", &[], &[]).is_err());
    }

    /// Yerel ağ keşfi: adres ve relay OLMADAN yalnız cihaz kimliğiyle, mDNS üzerinden bulunur
    /// (kayıtlı cihazın IP'si değişse de). Çoklu yayın gerektirdiği için CI'da atlanır.
    #[tokio::test]
    async fn gercek_mdns_kimlikle_bulunur() {
        async fn uc() -> Endpoint {
            Endpoint::builder(presets::Minimal)
                .relay_mode(RelayMode::Disabled)
                .alpns(vec![ALPN.to_vec()])
                .address_lookup(
                    iroh_mdns_address_lookup::MdnsAddressLookup::builder()
                        .service_name(MDNS_SERVISI),
                )
                .bind()
                .await
                .unwrap()
        }
        let h = uc().await;
        let e = h.clone();
        tokio::spawn(async move {
            if let Some(g) = e.accept().await {
                let c = g.await?;
                let (mut w, mut r) = c.accept_bi().await?;
                let v = r.read_to_end(64).await?;
                w.write_all(&v).await?;
                w.finish()?;
                c.closed().await;
            }
            Ok::<_, anyhow::Error>(())
        });
        let v = uc().await;
        let yalniz_kimlik = EndpointAddr::from_parts(h.id(), std::iter::empty());
        let c = baglan(&v, yalniz_kimlik, Duration::from_secs(20)).await.unwrap();
        let (mut w, mut r) = c.open_bi().await.unwrap();
        w.write_all(b"yerel").await.unwrap();
        w.finish().unwrap();
        assert_eq!(r.read_to_end(64).await.unwrap(), b"yerel");
    }

    #[test]
    fn baglanti_hatasi_kullaniciya_teknik_ayrinti_gostermez() {
        for m in [ULASILAMADI, GUVENLIK_HATASI] {
            assert!(!m.contains('('), "{m}");
            assert!(!m.contains("127.0.0.1") && !m.contains("deadline"), "{m}");
        }
    }

    #[test]
    fn genel_adres_ayrimi() {
        for a in ["192.168.0.110", "10.1.2.3", "172.28.96.1", "127.0.0.1", "169.254.1.1", "100.100.1.1", "::1", "fe80::1"] {
            assert!(!genel_mi(a.parse().unwrap()), "{a}");
        }
        for a in ["31.223.3.218", "8.8.8.8", "100.128.0.1", "2a02:e0::1"] {
            assert!(genel_mi(a.parse().unwrap()), "{a}");
        }
    }

    #[test]
    fn ortam_degiskeni_relay_zorlar() {
        assert_eq!(Kurulum::degerden(Some("1")), Kurulum::SadeceRelay);
        assert_eq!(Kurulum::degerden(Some(" 1 ")), Kurulum::SadeceRelay);
        assert_eq!(Kurulum::degerden(Some("0")), Kurulum::Internet);
        assert_eq!(Kurulum::degerden(None), Kurulum::Internet);
    }
}
