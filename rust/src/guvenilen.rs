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
}
