//! Linux network policy. Kept independent of platform APIs so boundary cases
//! can also be verified on the Windows development machine.
use std::net::{IpAddr, Ipv4Addr};

pub fn tailscale_mi(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.octets()[0] == 100 && ip.octets()[1] & 0xc0 == 64,
        IpAddr::V6(_) => false,
    }
}

pub fn baglama_ip(adresler: impl IntoIterator<Item = IpAddr>) -> IpAddr {
    adresler.into_iter().find(|ip| tailscale_mi(*ip))
        .unwrap_or(Ipv4Addr::LOCALHOST.into())
}

pub fn ozel_adresler(adresler: &[String]) -> Vec<String> {
    adresler.iter().filter(|a| {
        a.parse::<std::net::SocketAddr>().is_ok_and(|a| tailscale_mi(a.ip()))
    }).cloned().collect()
}

/// Read the address actually assigned to tailscale0. A CGNAT address supplied
/// in an environment variable is not evidence of a trusted interface.
#[cfg(target_os = "linux")]
pub fn sistem_baglama_ip() -> IpAddr {
    if std::path::Path::new("/sys/class/net/tailscale0").exists() {
        for program in ["/usr/sbin/ip", "/sbin/ip", "/usr/bin/ip", "/bin/ip"] {
            let Ok(cikti) = std::process::Command::new(program)
                .args(["-o", "-4", "addr", "show", "dev", "tailscale0"])
                .output() else { continue };
            if !cikti.status.success() { continue; }
            let metin = String::from_utf8_lossy(&cikti.stdout);
            let alanlar: Vec<_> = metin.split_whitespace().collect();
            return baglama_ip(alanlar.windows(2)
                .filter(|a| a[0] == "inet")
                .filter_map(|a| a[1].split('/').next()?.parse::<IpAddr>().ok()));
        }
    }
    Ipv4Addr::LOCALHOST.into()
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::net::SocketAddr;

    #[test]
    fn tailscale_yoksa_yalniz_loopback_dinler() {
        let ip = baglama_ip(["192.168.1.2".parse().unwrap(), "8.8.8.8".parse().unwrap()]);
        assert!(ip.is_loopback(), "Tailscale yokken dış arayüz açılmamalı");
        let soket = std::net::UdpSocket::bind(SocketAddr::new(ip, 0)).unwrap();
        assert!(!soket.local_addr().unwrap().ip().is_unspecified());
    }

    #[test]
    fn tailscale_fiziksel_agdan_once_secilir() {
        assert_eq!(
            baglama_ip(["192.168.1.2".parse().unwrap(), "100.100.1.2".parse().unwrap()]),
            "100.100.1.2".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn paylasimda_genel_lan_ve_ipv6_adresi_yok() {
        let adresler = ["8.8.8.8:7", "192.168.1.2:7", "[2001:4860::1]:7", "100.100.1.2:7"]
            .map(str::to_owned);
        assert_eq!(ozel_adresler(&adresler), vec!["100.100.1.2:7"]);
    }

    #[test]
    fn cgnat_sinirlari_ve_ipv6_mapped_adresler_kapali() {
        for ip in ["100.63.255.255", "100.128.0.0", "0.0.0.0", "::", "::ffff:100.64.0.1"] {
            assert!(!tailscale_mi(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["100.64.0.0", "100.127.255.255"] {
            assert!(tailscale_mi(ip.parse().unwrap()), "{ip}");
        }
    }
}
