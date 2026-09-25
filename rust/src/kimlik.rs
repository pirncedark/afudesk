//! Kalıcı cihaz kimliği ve TLS sertifikası.
use crate::ag::{self, Kimlik};
use anyhow::{Context, Result};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Serialize, Deserialize)]
struct Kalici {
    cihaz: String,
    ad: String,
    sertifika: Vec<u8>,
    anahtar: Vec<u8>,
}
pub struct CihazKimligi {
    pub cihaz: String,
    pub ad: String,
    pub tls: Kimlik,
}
pub fn yukle_veya_uret(yol: &Path, ad: &str) -> Result<CihazKimligi> {
    if let Ok(b) = fs::read(yol) {
        if let Ok(k) = serde_json::from_slice::<Kalici>(&b) {
            return Ok(CihazKimligi {
                cihaz: k.cihaz,
                ad: k.ad,
                tls: Kimlik {
                    parmak_izi: ag::parmak_izi(&k.sertifika),
                    sertifika: rustls::pki_types::CertificateDer::from(k.sertifika),
                    anahtar: rustls::pki_types::PrivatePkcs8KeyDer::from(k.anahtar),
                },
            });
        }
    }
    let tls = ag::yeni_kimlik()?;
    let mut id = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut id);
    let cihaz = id.iter().map(|x| format!("{x:02x}")).collect::<String>();
    let k = Kalici {
        cihaz: cihaz.clone(),
        ad: ad.into(),
        sertifika: tls.sertifika.to_vec(),
        anahtar: tls.anahtar.secret_pkcs8_der().to_vec(),
    };
    let parent = yol.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    use std::io::Write;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(&serde_json::to_vec(&k)?)?;
    tmp.persist(yol)
        .map_err(|e| e.error)
        .context("cihaz kimliği kaydedilemedi")?;
    Ok(CihazKimligi {
        cihaz,
        ad: ad.into(),
        tls,
    })
}
#[cfg(test)]
mod testler {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    #[test]
    fn cihaz_kimligi_kalici() {
        let p = std::env::temp_dir().join(format!(
            "afu-id-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let a = yukle_veya_uret(&p, "Bilgisayar").unwrap();
        let b = yukle_veya_uret(&p, "Yeni ad").unwrap();
        assert_eq!(a.cihaz, b.cihaz);
        assert_eq!(a.tls.parmak_izi, b.tls.parmak_izi);
        let _ = std::fs::remove_file(p);
    }
}
