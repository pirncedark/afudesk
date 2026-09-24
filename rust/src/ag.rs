//! QUIC uç noktaları. Sunucu her oturumda yeni kendinden imzalı sertifika üretir;
//! istemci sertifikayı CA ile değil, koddan gelen SHA-256 parmak iziyle doğrular.
use crate::protokol::ALPN;
use anyhow::{Context, Result};
use quinn::crypto::rustls::{QuicClientConfig, QuicServerConfig};
use rustls::{
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::CryptoProvider,
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime},
    DigitallySignedStruct, SignatureScheme,
};
use sha2::{Digest, Sha256};
use std::{net::SocketAddr, sync::Arc, time::Duration};

/// RTT'si en düşük bağlantı adayının dizinini döndürür.
pub fn en_iyi_yol(adaylar: &[(usize, Duration)]) -> Option<usize> {
    adaylar.iter().min_by_key(|(_, rtt)| *rtt).map(|(indeks, _)| *indeks)
}

#[cfg(test)]
mod yol_testleri {
    use super::*;

    #[test]
    fn en_dusuk_rtt_yolu_secer() {
        assert_eq!(en_iyi_yol(&[(0, Duration::from_millis(80)), (1, Duration::from_millis(12))]), Some(1));
        assert_eq!(en_iyi_yol(&[]), None);
    }
}

pub struct Kimlik {
    pub sertifika: CertificateDer<'static>,
    pub anahtar: PrivatePkcs8KeyDer<'static>,
    pub parmak_izi: String,
}

pub fn parmak_izi(der: &[u8]) -> String {
    Sha256::digest(der).iter().map(|b| format!("{b:02x}")).collect()
}

pub fn yeni_kimlik() -> Result<Kimlik> {
    let s = rcgen::generate_simple_self_signed(vec!["afudesk".to_owned()])?;
    let sertifika = CertificateDer::from(s.cert.der().to_vec());
    let anahtar = PrivatePkcs8KeyDer::from(s.key_pair.serialize_der());
    let parmak_izi = parmak_izi(&sertifika);
    Ok(Kimlik { sertifika, anahtar, parmak_izi })
}

fn saglayici() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

fn tasima() -> Arc<quinn::TransportConfig> {
    let mut t = quinn::TransportConfig::default();
    t.keep_alive_interval(Some(Duration::from_secs(5)));
    t.max_idle_timeout(Some(Duration::from_secs(20).try_into().expect("süre")));
    Arc::new(t)
}

/// `adres` örn. "0.0.0.0:0" (rastgele port) ya da "[::]:47000".
pub fn sunucu(kimlik: &Kimlik, adres: SocketAddr) -> Result<quinn::Endpoint> {
    let mut tls = rustls::ServerConfig::builder_with_provider(saglayici())
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(
            vec![kimlik.sertifika.clone()],
            PrivateKeyDer::Pkcs8(kimlik.anahtar.clone_key()),
        )?;
    tls.alpn_protocols = vec![ALPN.to_vec()];
    let mut cfg = quinn::ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(tls)?));
    cfg.transport_config(tasima());
    Ok(quinn::Endpoint::server(cfg, adres)?)
}

#[derive(Debug)]
struct ParmakIziDogrulayici {
    beklenen: String,
    saglayici: Arc<CryptoProvider>,
}

impl ServerCertVerifier for ParmakIziDogrulayici {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _ara: &[CertificateDer<'_>],
        _ad: &ServerName<'_>,
        _ocsp: &[u8],
        _simdi: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        if parmak_izi(end_entity) == self.beklenen {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General("sertifika parmak izi koddakiyle uyuşmuyor".into()))
        }
    }

    fn verify_tls12_signature(
        &self,
        _m: &[u8],
        _c: &CertificateDer<'_>,
        _d: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Err(rustls::Error::General("TLS 1.2 desteklenmiyor".into()))
    }

    fn verify_tls13_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        d: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(m, c, d, &self.saglayici.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.saglayici.signature_verification_algorithms.supported_schemes()
    }
}

pub fn istemci_yapilandirmasi(parmak_izi: &str) -> Result<quinn::ClientConfig> {
    let p = saglayici();
    let mut tls = rustls::ClientConfig::builder_with_provider(p.clone())
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(ParmakIziDogrulayici {
            beklenen: parmak_izi.to_lowercase(),
            saglayici: p,
        }))
        .with_no_client_auth();
    tls.alpn_protocols = vec![ALPN.to_vec()];
    let mut cfg = quinn::ClientConfig::new(Arc::new(QuicClientConfig::try_from(tls)?));
    cfg.transport_config(tasima());
    Ok(cfg)
}

/// Adresleri paralel dener; ilk başarılı bağlantı kazanır.
pub async fn baglan(adresler: &[String], parmak_izi: &str, sure: Duration) -> Result<quinn::Connection> {
    anyhow::ensure!(!adresler.is_empty(), "Kodda adres yok.");
    let cfg = istemci_yapilandirmasi(parmak_izi)?;
    let mut gorevler = tokio::task::JoinSet::new();
    let mut son_hata = String::from("Karşı tarafa ulaşılamadı.");
    for a in adresler {
        let Ok(hedef) = a.parse::<SocketAddr>() else { continue };
        let yerel: SocketAddr = if hedef.is_ipv6() { "[::]:0" } else { "0.0.0.0:0" }.parse()?;
        let ep = match quinn::Endpoint::client(yerel) {
            Ok(e) => e,
            Err(e) => {
                son_hata = e.to_string();
                continue;
            }
        };
        let cfg = cfg.clone();
        gorevler.spawn(async move {
            let c = ep.connect_with(cfg, hedef, "afudesk")?;
            let b = tokio::time::timeout(sure, c).await.context("zaman aşımı")??;
            Ok::<_, anyhow::Error>(b)
        });
    }
    let mut basarililar = Vec::new();
    let mut secim_sonu = None;
    while !gorevler.is_empty() {
        let sonuc = if let Some(sinir) = secim_sonu {
            match tokio::time::timeout_at(sinir, gorevler.join_next()).await {
                Ok(s) => s,
                Err(_) => break,
            }
        } else {
            gorevler.join_next().await
        };
        let Some(s) = sonuc else { break };
        match s {
            Ok(Ok(b)) => {
                if secim_sonu.is_none() {
                    secim_sonu = Some(tokio::time::Instant::now() + Duration::from_millis(300));
                }
                basarililar.push(b);
            }
            Ok(Err(e)) => {
                son_hata = format!("{e:#}");
            }
            Err(e) => {
                son_hata = e.to_string();
            }
        }
    }
    if !basarililar.is_empty() {
        let rttler: Vec<_> = basarililar.iter().enumerate().map(|(i, c)| (i, c.rtt())).collect();
        let secilen = en_iyi_yol(&rttler).expect("bağlantı adayı var");
        let kazanan = basarililar.swap_remove(secilen);
        for aday in basarililar {
            aday.close(0u32.into(), b"daha dusuk RTT yolu secildi");
        }
        gorevler.abort_all();
        return Ok(kazanan);
    }
    if son_hata.contains("parmak izi") {
        anyhow::bail!("Güvenlik kontrolü başarısız: karşıdaki cihaz koddaki cihaz değil.");
    }
    anyhow::bail!(
        "Karşı tarafa ulaşılamadı. Aynı ağda değilseniz, bağlantı veren tarafın modeminde UPnP açık olmalı. ({son_hata})"
    )
}

#[cfg(test)]
mod testler {
    use super::*;

    async fn yankici(k: &Kimlik) -> (quinn::Endpoint, String) {
        let ep = sunucu(k, "127.0.0.1:0".parse().unwrap()).unwrap();
        let adres = ep.local_addr().unwrap().to_string();
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
        (ep, adres)
    }

    #[tokio::test]
    async fn dogru_parmak_izi_baglanir() {
        let k = yeni_kimlik().unwrap();
        let (_ep, adres) = yankici(&k).await;
        let c = baglan(&[adres], &k.parmak_izi, Duration::from_secs(5)).await.unwrap();
        let (mut w, mut r) = c.open_bi().await.unwrap();
        w.write_all(b"merhaba").await.unwrap();
        w.finish().unwrap();
        assert_eq!(r.read_to_end(1024).await.unwrap(), b"merhaba");
    }

    #[tokio::test]
    async fn yanlis_parmak_izi_reddedilir() {
        let k = yeni_kimlik().unwrap();
        let sahte = yeni_kimlik().unwrap();
        let (_ep, adres) = yankici(&k).await;
        let h = baglan(&[adres], &sahte.parmak_izi, Duration::from_secs(5)).await.unwrap_err();
        assert!(h.to_string().starts_with("Güvenlik kontrolü başarısız"), "{h}");
    }

    #[tokio::test]
    async fn olu_adres_varsa_digeri_kazanir() {
        let k = yeni_kimlik().unwrap();
        let (_ep, adres) = yankici(&k).await;
        let adresler = vec!["127.0.0.1:9".to_owned(), "gecersiz".to_owned(), adres];
        assert!(baglan(&adresler, &k.parmak_izi, Duration::from_secs(5)).await.is_ok());
    }

    #[tokio::test]
    async fn hic_ulasilamazsa_anlasilir_hata() {
        let k = yeni_kimlik().unwrap();
        let h = baglan(&["127.0.0.1:9".to_owned()], &k.parmak_izi, Duration::from_secs(2))
            .await
            .unwrap_err();
        assert!(h.to_string().starts_with("Karşı tarafa ulaşılamadı"), "{h}");
        assert!(baglan(&[], &k.parmak_izi, Duration::from_secs(1)).await.is_err());
    }
}
