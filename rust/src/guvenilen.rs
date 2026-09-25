//! Yerel güvenilir cihaz kayıtları. JSON dosyası atomik değiştirilir.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostKaydi {
    pub izleyici_kimlik: String,
    pub ad: String,
    pub jeton_sha256: String,
    pub eklenme: i64,
    pub son_gorulme: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IzleyiciKaydi {
    pub host_kimlik: String,
    pub ad: String,
    pub son_adresler: Vec<String>,
    pub host_parmak_izi: String,
    pub jeton: String,
    pub son_gorulme: i64,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Kayitlar {
    pub hostlar: Vec<HostKaydi>,
    pub izleyiciler: Vec<IzleyiciKaydi>,
}
pub fn jeton_ozeti(j: &str) -> String {
    hex(&Sha256::digest(j.as_bytes()))
}
/// Jetonu karşılaştırmadan önce özetler; baytların tamamını sabit sürede karşılaştırır.
pub fn jeton_dogrula(jeton: &str, ozet: &str) -> bool {
    let hesaplanan = jeton_ozeti(jeton);
    hesaplanan.len() == ozet.len()
        && hesaplanan
            .bytes()
            .zip(ozet.bytes())
            .fold(0u8, |fark, (a, b)| fark | (a ^ b))
            == 0
}

/// Kriptografik rastgele 256 bit eşleştirme jetonu.
pub fn yeni_jeton() -> String {
    use rand::RngCore;
    let mut baytlar = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut baytlar);
    hex(&baytlar)
}

/// Kayıtlı izleyicinin jeton özetini kaydeder veya mevcut eşleşmenin üzerine yazar.
pub fn hosta_ekle(kayitlar: &mut Kayitlar, kayit: HostKaydi) {
    kayitlar.hostlar.retain(|eski| eski.izleyici_kimlik != kayit.izleyici_kimlik);
    kayitlar.hostlar.push(kayit);
}

/// Host kullanıcısının kaldırdığı eşleşmeyi bellekten çıkarır.
pub fn hosttan_kaldir(kayitlar: &mut Kayitlar, izleyici_kimlik: &str) -> bool {
    let once = kayitlar.hostlar.len();
    kayitlar.hostlar.retain(|k| k.izleyici_kimlik != izleyici_kimlik);
    once != kayitlar.hostlar.len()
}

/// İzleyicinin kendi listesinden bir host kaydını kaldırır.
pub fn hostu_unut(kayitlar: &mut Kayitlar, host_kimlik: &str) -> bool {
    let once = kayitlar.izleyiciler.len();
    kayitlar.izleyiciler.retain(|k| k.host_kimlik != host_kimlik);
    once != kayitlar.izleyiciler.len()
}
fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
pub fn oku(yol: &Path) -> Kayitlar {
    fs::read(yol)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}
pub fn yaz(yol: &Path, k: &Kayitlar) -> std::io::Result<()> {
    let parent = yol.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let bytes = serde_json::to_vec(k).map_err(std::io::Error::other)?;
    use std::io::Write;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(&bytes)?;
    tmp.persist(yol).map(|_| ()).map_err(|e| e.error)
}
#[cfg(test)]
pub mod testler {
    use super::*;
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };
    fn path(n: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "afudesk-{n}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
    #[test]
    fn kayit_gidis_donus() {
        let p = path("roundtrip");
        let k = Kayitlar {
            hostlar: vec![HostKaydi {
                izleyici_kimlik: "aa".into(),
                ad: "Telefon".into(),
                jeton_sha256: jeton_ozeti("secret"),
                eklenme: 1,
                son_gorulme: 2,
            }],
            ..Default::default()
        };
        yaz(&p, &k).unwrap();
        assert_eq!(oku(&p), k);
        let _ = fs::remove_file(p);
    }
    #[test]
    fn atomik_yazma() {
        let p = path("atomic");
        let k = Kayitlar::default();
        yaz(&p, &k).unwrap();
        let k2 = Kayitlar {
            izleyiciler: vec![IzleyiciKaydi {
                host_kimlik: "bb".into(),
                ad: "Laptop".into(),
                son_adresler: vec!["127.0.0.1:1".into()],
                host_parmak_izi: "cc".into(),
                jeton: "dd".into(),
                son_gorulme: 3,
            }],
            ..Default::default()
        };
        yaz(&p, &k2).unwrap();
        assert_eq!(oku(&p), k2);
        let _ = fs::remove_file(p);
    }

    #[test]
    fn jeton_dogrulamasi() {
        let jeton = yeni_jeton();
        assert_eq!(jeton.len(), 64);
        assert!(jeton_dogrula(&jeton, &jeton_ozeti(&jeton)));
        assert!(!jeton_dogrula("yanlis", &jeton_ozeti(&jeton)));
    }

    #[test]
    fn guvenilen_cihaz_kaldirilir_ve_unutulur() {
        let mut kayitlar = Kayitlar {
            hostlar: vec![HostKaydi {
                izleyici_kimlik: "izleyici-1".into(),
                ad: "Telefon".into(),
                jeton_sha256: jeton_ozeti("jeton"),
                eklenme: 1,
                son_gorulme: 1,
            }],
            izleyiciler: vec![IzleyiciKaydi {
                host_kimlik: "host-1".into(),
                ad: "Bilgisayar".into(),
                son_adresler: vec![],
                host_parmak_izi: "parmak-izi".into(),
                jeton: "jeton".into(),
                son_gorulme: 1,
            }],
        };
        assert!(hosttan_kaldir(&mut kayitlar, "izleyici-1"));
        assert!(!hosttan_kaldir(&mut kayitlar, "izleyici-1"));
        assert!(hostu_unut(&mut kayitlar, "host-1"));
        assert!(kayitlar.hostlar.is_empty() && kayitlar.izleyiciler.is_empty());
    }
}
