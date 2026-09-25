//! Kalıcı cihaz kimliği: iroh gizli anahtarı. Açık anahtar (uç kimliği) cihazın kimliğidir
//! ve her bağlantıda el sıkışmada doğrulanır; kayıtlı cihazlar bu kimlikle tanınır.
//! Host ve izleyici ayrı anahtar kullanır (aynı anda ikisi de açık olabilir).
use anyhow::{Context, Result};
use iroh::{EndpointId, SecretKey};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

pub const HOST_DOSYASI: &str = "kimlik_host.json";
pub const IZLEYICI_DOSYASI: &str = "kimlik_izleyici.json";

#[derive(Serialize, Deserialize)]
struct Kalici {
    v: u8,
    /// 32 bayt gizli anahtar (hex).
    gizli_anahtar: String,
}

pub struct CihazKimligi {
    pub gizli: SecretKey,
}

impl CihazKimligi {
    pub fn kimlik(&self) -> EndpointId {
        self.gizli.public()
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn hex_coz(s: &str) -> Option<[u8; 32]> {
    let s = s.trim();
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// Dosyada kimlik varsa yükler, yoksa (ya da bozuksa) yenisini üretip atomik olarak yazar.
pub fn yukle_veya_uret(yol: &Path) -> Result<CihazKimligi> {
    if let Ok(b) = fs::read(yol) {
        if let Some(anahtar) = serde_json::from_slice::<Kalici>(&b)
            .ok()
            .and_then(|k| hex_coz(&k.gizli_anahtar))
        {
            return Ok(CihazKimligi {
                gizli: SecretKey::from_bytes(&anahtar),
            });
        }
    }
    let gizli = SecretKey::generate();
    let k = Kalici {
        v: 1,
        gizli_anahtar: hex(&gizli.to_bytes()),
    };
    crate::guvenilen::atomik_yaz(yol, &serde_json::to_vec(&k)?)
        .context("cihaz kimliği kaydedilemedi")?;
    Ok(CihazKimligi { gizli })
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn cihaz_kimligi_kalici() {
        let klasor = crate::guvenilen::testler::gecici_yol("kimlik");
        let p = klasor.join(HOST_DOSYASI);
        let a = yukle_veya_uret(&p).unwrap();
        let b = yukle_veya_uret(&p).unwrap();
        assert_eq!(a.kimlik(), b.kimlik(), "ikinci yüklemede aynı kimlik");
        let c = yukle_veya_uret(&klasor.join(IZLEYICI_DOSYASI)).unwrap();
        assert_ne!(a.kimlik(), c.kimlik(), "host ve izleyici ayrı kimlik");
        // Bozuk dosya: yeni kimlik üretilir, uygulama çökmez.
        std::fs::write(&p, b"bozuk").unwrap();
        let d = yukle_veya_uret(&p).unwrap();
        assert_ne!(a.kimlik(), d.kimlik());
        let _ = std::fs::remove_dir_all(klasor);
    }
}
