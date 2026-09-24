//! Bağlantı veren cihazın ulaşılabilir adresleri: LAN, UPnP ile açılmış genel adres, IPv6.
use igd_next::{aio::tokio as igd, PortMappingProtocol, SearchOptions};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};

/// UPnP eşlemesi; `kapat` ile modemdeki port geri kapatılır.
pub struct UpnpEslemesi {
    gecit: igd_next::aio::Gateway<igd_next::aio::tokio::Tokio>,
    dis_port: u16,
    pub dis_adres: SocketAddr,
}

impl UpnpEslemesi {
    pub async fn kapat(self) {
        let _ = self
            .gecit
            .remove_port(PortMappingProtocol::UDP, self.dis_port)
            .await;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpnpDurum {
    Acik(String),
    Yok(String),
    Kapali,
}

impl UpnpDurum {
    pub fn aciklama(&self) -> String {
        match self {
            UpnpDurum::Acik(a) => format!("İnternetten ulaşılabilir ({a})"),
            UpnpDurum::Yok(s) => format!("Yalnız aynı ağdan ulaşılabilir — {s}"),
            UpnpDurum::Kapali => "Yalnız aynı ağdan ulaşılabilir".into(),
        }
    }
}

pub fn yerel_ipv4() -> Option<Ipv4Addr> {
    match local_ip_address::local_ip() {
        Ok(IpAddr::V4(a)) if !a.is_loopback() => Some(a),
        _ => None,
    }
}

/// Genel (küresel tek yayın) IPv6 adresleri: 2000::/3.
pub fn genel_ipv6() -> Vec<std::net::Ipv6Addr> {
    let Ok(liste) = local_ip_address::list_afinet_netifas() else {
        return vec![];
    };
    let mut v: Vec<_> = liste
        .into_iter()
        .filter_map(|(_, ip)| match ip {
            IpAddr::V6(a) if a.segments()[0] & 0xe000 == 0x2000 => Some(a),
            _ => None,
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

pub async fn upnp_ac(yerel: Ipv4Addr, port: u16) -> Result<UpnpEslemesi, String> {
    let secenek = SearchOptions {
        timeout: Some(Duration::from_secs(3)),
        ..Default::default()
    };
    let gecit = igd::search_gateway(secenek)
        .await
        .map_err(|_| "modemde UPnP kapalı ya da desteklenmiyor".to_owned())?;
    let dis_ip = gecit
        .get_external_ip()
        .await
        .map_err(|e| format!("dış IP alınamadı: {e}"))?;
    if !genel_mi(dis_ip) {
        return Err(format!(
            "modem dış adresi özel ağ ({dis_ip}); operatör CGNAT kullanıyor olabilir"
        ));
    }
    let yerel_adres = SocketAddr::V4(SocketAddrV4::new(yerel, port));
    let dis_port = match gecit
        .add_port(PortMappingProtocol::UDP, port, yerel_adres, 7200, "AfuDesk")
        .await
    {
        Ok(()) => port,
        Err(_) => gecit
            .add_any_port(PortMappingProtocol::UDP, yerel_adres, 7200, "AfuDesk")
            .await
            .map_err(|e| format!("port açılamadı: {e}"))?,
    };
    Ok(UpnpEslemesi {
        gecit,
        dis_port,
        dis_adres: SocketAddr::new(dis_ip, dis_port),
    })
}

fn genel_mi(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(a) => {
            !(a.is_private() || a.is_loopback() || a.is_link_local() || a.is_unspecified()
                // 100.64.0.0/10 — CGNAT
                || (a.octets()[0] == 100 && (a.octets()[1] & 0xc0) == 64))
        }
        IpAddr::V6(a) => a.segments()[0] & 0xe000 == 0x2000,
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn genel_adres_ayrimi() {
        assert!(genel_mi("85.105.1.1".parse().unwrap()));
        assert!(!genel_mi("192.168.1.1".parse().unwrap()));
        assert!(!genel_mi("10.0.0.1".parse().unwrap()));
        assert!(!genel_mi("100.72.3.4".parse().unwrap()), "CGNAT");
        assert!(genel_mi("100.200.3.4".parse().unwrap()));
        assert!(genel_mi("2a02:e0::1".parse().unwrap()));
        assert!(!genel_mi("fe80::1".parse().unwrap()));
    }

    #[test]
    fn yerel_ipv4_dongu_degil() {
        if let Some(a) = yerel_ipv4() {
            assert!(!a.is_loopback());
        }
    }
}
