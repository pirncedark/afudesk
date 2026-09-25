//! Kayıtlı (güvenilen) cihazlar. İlk kez kodla bağlanıp onaylanan izleyiciye host bir
//! eşleşme jetonu verir; sonraki bağlantılarda kod/parola gerekmez ama host her seferinde
//! yine onay verir. Host yalnız jetonun SHA-256 özetini saklar; jeton izleyicinin cihaz
//! kimliğine bağlıdır (başka cihaz jetonu ele geçirse de kullanamaz).
//!
//! Kayıtlar tek JSON dosyasında (`kayitlar.json`) tutulur ve atomik yazılır (geçici dosya +
//! yeniden adlandırma): yazma yarıda kesilse de dosya bozulmaz.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

pub const KAYIT_DOSYASI: &str = "kayitlar.json";
/// Host bu izleyiciyi listesinden kaldırmışsa izleyiciye giden cevap.
pub const KALDIRILDI: &str = "Bu cihaz sizi kaldırdı. Yeniden bağlanmak için kod gerekir.";
/// Jeton/kimlik uyuşmuyorsa.
pub const GECERSIZ: &str = "Kayıtlı bağlantı geçersiz. Yeniden bağlanmak için kod gerekir.";

/// Host tarafı: güvenilen izleyici.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostKaydi {
    /// İzleyicinin uç kimliği (açık anahtar, hex).
    pub izleyici_kimlik: String,
    pub ad: String,
    pub jeton_sha256: String,
    pub eklenme: i64,
    pub son_gorulme: i64,
}

/// İzleyici tarafı: bağlanılabilecek kayıtlı host.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IzleyiciKaydi {
    /// Host'un uç kimliği (açık anahtar, hex); bağlantıda el sıkışmada doğrulanır.
    pub host_kimlik: String,
    pub ad: String,
    pub son_adresler: Vec<String>,
    #[serde(default)]
    pub relaylar: Vec<String>,
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

/// Jetonu özetleyip sabit sürede karşılaştırır.
pub fn jeton_dogrula(jeton: &str, ozet: &str) -> bool {
    let hesaplanan = jeton_ozeti(jeton);
    hesaplanan.len() == ozet.len()
        && hesaplanan
            .bytes()
            .zip(ozet.bytes())
            .fold(0u8, |fark, (a, b)| fark | (a ^ b))
            == 0
}

/// Kriptografik rastgele 256 bit eşleşme jetonu.
pub fn yeni_jeton() -> String {
    use rand::RngCore;
    let mut b = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut b);
    hex(&b)
}

/// Host: izleyiciyi ekler (varsa üzerine yazar).
pub fn hosta_ekle(k: &mut Kayitlar, kayit: HostKaydi) {
    k.hostlar.retain(|e| e.izleyici_kimlik != kayit.izleyici_kimlik);
    k.hostlar.push(kayit);
}

/// Host: kullanıcının kaldırdığı izleyiciyi siler.
pub fn hosttan_kaldir(k: &mut Kayitlar, izleyici_kimlik: &str) -> bool {
    let once = k.hostlar.len();
    k.hostlar.retain(|e| e.izleyici_kimlik != izleyici_kimlik);
    once != k.hostlar.len()
}

/// İzleyici: host kaydını ekler (varsa üzerine yazar).
pub fn izleyiciye_ekle(k: &mut Kayitlar, kayit: IzleyiciKaydi) {
    k.izleyiciler.retain(|e| e.host_kimlik != kayit.host_kimlik);
    k.izleyiciler.push(kayit);
}

/// İzleyici: kendi listesinden host'u unutur.
pub fn hostu_unut(k: &mut Kayitlar, host_kimlik: &str) -> bool {
    let once = k.izleyiciler.len();
    k.izleyiciler.retain(|e| e.host_kimlik != host_kimlik);
    once != k.izleyiciler.len()
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

/// Geçici dosyaya yazıp yerine taşır: yazma yarıda kalsa da eski dosya bozulmaz.
pub fn atomik_yaz(yol: &Path, veri: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let klasor = yol.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(klasor)?;
    let mut tmp = tempfile::NamedTempFile::new_in(klasor)?;
    tmp.write_all(veri)?;
    tmp.as_file().sync_all()?;
    tmp.persist(yol).map(|_| ()).map_err(|e| e.error)
}

pub fn yaz(yol: &Path, k: &Kayitlar) -> std::io::Result<()> {
    atomik_yaz(yol, &serde_json::to_vec_pretty(k).map_err(std::io::Error::other)?)
}

/// Aynı süreçte host ve izleyici aynı dosyayı değiştirebilir: oku-değiştir-yaz tek kilitle.
static KILIT: Mutex<()> = Mutex::new(());

pub fn guncelle<T>(yol: &Path, f: impl FnOnce(&mut Kayitlar) -> T) -> std::io::Result<T> {
    let _k = KILIT.lock().unwrap_or_else(|e| e.into_inner());
    let mut k = oku(yol);
    let sonuc = f(&mut k);
    yaz(yol, &k)?;
    Ok(sonuc)
}

pub fn kayit_yolu(veri_klasoru: &Path) -> PathBuf {
    veri_klasoru.join(KAYIT_DOSYASI)
}

pub fn simdi() -> i64 {
    crate::kod::simdi()
}

#[cfg(test)]
pub mod testler {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn gecici_yol(on: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "afudesk-{on}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn ornek() -> Kayitlar {
        Kayitlar {
            hostlar: vec![HostKaydi {
                izleyici_kimlik: "aa".into(),
                ad: "Telefon".into(),
                jeton_sha256: jeton_ozeti("gizli"),
                eklenme: 1,
                son_gorulme: 2,
            }],
            izleyiciler: vec![IzleyiciKaydi {
                host_kimlik: "bb".into(),
                ad: "Laptop".into(),
                son_adresler: vec!["127.0.0.1:1".into()],
                relaylar: vec![],
                jeton: "dd".into(),
                son_gorulme: 3,
            }],
        }
    }

    #[test]
    fn kayit_gidis_donus() {
        let p = gecici_yol("kayit").join(KAYIT_DOSYASI);
        yaz(&p, &ornek()).unwrap();
        assert_eq!(oku(&p), ornek());
        // Dosya yoksa ya da bozuksa boş liste (çökme yok).
        assert_eq!(oku(&p.with_extension("yok")), Kayitlar::default());
        fs::write(&p, b"{bozuk").unwrap();
        assert_eq!(oku(&p), Kayitlar::default());
        let _ = fs::remove_dir_all(p.parent().unwrap());
    }

    #[test]
    fn atomik_yazma() {
        let p = gecici_yol("atomik").join(KAYIT_DOSYASI);
        yaz(&p, &Kayitlar::default()).unwrap();
        yaz(&p, &ornek()).unwrap();
        assert_eq!(oku(&p), ornek());
        // Geride geçici dosya kalmaz.
        let dosyalar: Vec<_> = fs::read_dir(p.parent().unwrap()).unwrap().collect();
        assert_eq!(dosyalar.len(), 1, "yalnız kayit dosyası kalmalı");
        let _ = fs::remove_dir_all(p.parent().unwrap());
    }

    #[test]
    fn jeton_dogrulamasi() {
        let j = yeni_jeton();
        assert_eq!(j.len(), 64);
        assert_ne!(j, yeni_jeton());
        assert!(jeton_dogrula(&j, &jeton_ozeti(&j)));
        assert!(!jeton_dogrula("yanlis", &jeton_ozeti(&j)));
        assert!(!jeton_dogrula(&j, ""));
    }

    #[test]
    fn eszamanli_guncelleme_kayit_kaybetmez() {
        let p = gecici_yol("eszamanli").join(KAYIT_DOSYASI);
        let ipler: Vec<_> = (0..8)
            .map(|i| {
                let p = p.clone();
                std::thread::spawn(move || {
                    guncelle(&p, |k| {
                        hosta_ekle(
                            k,
                            HostKaydi {
                                izleyici_kimlik: format!("iz{i}"),
                                ad: String::new(),
                                jeton_sha256: String::new(),
                                eklenme: 0,
                                son_gorulme: 0,
                            },
                        )
                    })
                    .unwrap()
                })
            })
            .collect();
        for i in ipler {
            i.join().unwrap();
        }
        assert_eq!(oku(&p).hostlar.len(), 8);
        let _ = fs::remove_dir_all(p.parent().unwrap());
    }

    #[test]
    fn guvenilen_cihaz_kaldirilir_ve_unutulur() {
        let mut k = ornek();
        assert!(hosttan_kaldir(&mut k, "aa"));
        assert!(!hosttan_kaldir(&mut k, "aa"));
        assert!(hostu_unut(&mut k, "bb"));
        assert!(k.hostlar.is_empty() && k.izleyiciler.is_empty());
    }
}
