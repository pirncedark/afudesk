//! Bağlantı kodu: `AFU2.<base64url(tuz[16] | nonce[24] | XChaCha20-Poly1305(JSON))>`.
//! Anahtar Argon2id(parola, tuz). Kodu ele geçiren biri parola olmadan adresleri göremez,
//! içeriği değiştiremez (sertifika parmak izi burada taşındığı için MITM de engellenir).
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    XChaCha20Poly1305, XNonce,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};

pub const GECERLILIK_SN: i64 = 600;
const ONEK: &str = "AFU2.";
const TUZ: usize = 16;
const NONCE: usize = 24;
const ETIKET: usize = 16;
const AAD: &[u8] = b"AFU2|davet";
const AZAMI_KOD: usize = 16 * 1024;

/// Kodun içindeki yük.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Davet {
    pub v: u8,
    /// Bağlantı veren tarafın görünen adı.
    pub ad: String,
    /// Denenecek adresler, öncelik sırasıyla ("192.168.1.5:47000", "[2a02::1]:47000").
    pub adresler: Vec<String>,
    /// Sunucu sertifikasının SHA-256 parmak izi (hex).
    pub parmak_izi: String,
    /// Tek kullanımlık bilet (hex); bağlanan taraf ilk mesajda gönderir.
    pub bilet: String,
    /// Unix saniye.
    pub bitis: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum KodHata {
    #[error("Kod eksik ya da bozuk; tamamını kopyaladığından emin ol.")]
    Bozuk,
    #[error("Parola yanlış.")]
    ParolaYanlis,
    #[error("Parola boş olamaz.")]
    ParolaBos,
    #[error("Kodun süresi dolmuş, yeni kod iste.")]
    SuresiDolmus,
    #[error("Kod farklı bir AfuDesk sürümünden; iki taraf da güncellemeli.")]
    Surum,
}

fn anahtar(parola: &str, tuz: &[u8]) -> Result<[u8; 32], KodHata> {
    let parola = parola.trim();
    if parola.is_empty() {
        return Err(KodHata::ParolaBos);
    }
    // 64 MiB, 3 tur, 1 kulvar.
    let params = Params::new(64 * 1024, 3, 1, Some(32)).map_err(|_| KodHata::Bozuk)?;
    let mut k = [0u8; 32];
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(parola.as_bytes(), tuz, &mut k)
        .map_err(|_| KodHata::Bozuk)?;
    Ok(k)
}

pub fn kodla(d: &Davet, parola: &str) -> Result<String, KodHata> {
    let mut tuz = [0u8; TUZ];
    let mut nonce = [0u8; NONCE];
    rand::thread_rng().fill_bytes(&mut tuz);
    rand::thread_rng().fill_bytes(&mut nonce);
    let k = anahtar(parola, &tuz)?;
    let json = serde_json::to_vec(d).map_err(|_| KodHata::Bozuk)?;
    let sifreli = XChaCha20Poly1305::new(&k.into())
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &json,
                aad: AAD,
            },
        )
        .map_err(|_| KodHata::Bozuk)?;
    let mut ham = Vec::with_capacity(TUZ + NONCE + sifreli.len());
    ham.extend_from_slice(&tuz);
    ham.extend_from_slice(&nonce);
    ham.extend_from_slice(&sifreli);
    Ok(format!("{ONEK}{}", URL_SAFE_NO_PAD.encode(ham)))
}

/// Kodu çözer; boşluk ve satır sonları (mesajlaşma uygulamalarından kopyalama) yok sayılır.
pub fn coz(kod: &str, parola: &str, simdi: i64) -> Result<Davet, KodHata> {
    if parola.trim().is_empty() {
        return Err(KodHata::ParolaBos);
    }
    let temiz: String = kod.chars().filter(|c| !c.is_whitespace()).collect();
    if temiz.len() > AZAMI_KOD {
        return Err(KodHata::Bozuk);
    }
    if temiz.starts_with("AFU") && !temiz.starts_with(ONEK) {
        return Err(KodHata::Surum);
    }
    let govde = temiz.strip_prefix(ONEK).ok_or(KodHata::Bozuk)?;
    let ham = URL_SAFE_NO_PAD.decode(govde).map_err(|_| KodHata::Bozuk)?;
    if ham.len() <= TUZ + NONCE + ETIKET {
        return Err(KodHata::Bozuk);
    }
    let k = anahtar(parola, &ham[..TUZ])?;
    let json = XChaCha20Poly1305::new(&k.into())
        .decrypt(
            XNonce::from_slice(&ham[TUZ..TUZ + NONCE]),
            Payload {
                msg: &ham[TUZ + NONCE..],
                aad: AAD,
            },
        )
        .map_err(|_| KodHata::ParolaYanlis)?;
    let d: Davet = serde_json::from_slice(&json).map_err(|_| KodHata::Bozuk)?;
    if d.v != 2 {
        return Err(KodHata::Surum);
    }
    if simdi > d.bitis {
        return Err(KodHata::SuresiDolmus);
    }
    Ok(d)
}

/// 6 haneli rakam parola.
pub fn yeni_parola() -> String {
    format!("{:06}", rand::random::<u32>() % 1_000_000)
}

/// 32 hex karakter rastgele bilet.
pub fn yeni_bilet() -> String {
    let mut b = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn simdi() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod testler {
    use super::*;

    fn ornek() -> Davet {
        Davet {
            v: 2,
            ad: "Ali'nin PC'si".into(),
            adresler: vec!["192.168.1.5:47000".into(), "[2a02:e0::1]:47000".into()],
            parmak_izi: "ab".repeat(32),
            bilet: yeni_bilet(),
            bitis: 1_000 + GECERLILIK_SN,
        }
    }

    #[test]
    fn gidis_donus() {
        let d = ornek();
        let k = kodla(&d, "123456").unwrap();
        assert!(k.starts_with("AFU2."));
        assert_eq!(coz(&k, "123456", 1_000).unwrap(), d);
    }

    #[test]
    fn kod_kisa_kalir() {
        // WhatsApp'ta tek parça kopyalanabilmeli.
        let k = kodla(&ornek(), "123456").unwrap();
        assert!(k.len() < 500, "kod {} karakter", k.len());
    }

    #[test]
    fn parola_yanlis() {
        let k = kodla(&ornek(), "123456").unwrap();
        assert_eq!(coz(&k, "654321", 1_000), Err(KodHata::ParolaYanlis));
    }

    #[test]
    fn bos_parola() {
        assert_eq!(kodla(&ornek(), "  "), Err(KodHata::ParolaBos));
        let k = kodla(&ornek(), "123456").unwrap();
        assert_eq!(coz(&k, "", 1_000), Err(KodHata::ParolaBos));
    }

    #[test]
    fn suresi_dolmus_ve_sinir() {
        let k = kodla(&ornek(), "123456").unwrap();
        assert!(coz(&k, "123456", 1_000 + GECERLILIK_SN).is_ok());
        assert_eq!(
            coz(&k, "123456", 1_001 + GECERLILIK_SN),
            Err(KodHata::SuresiDolmus)
        );
    }

    #[test]
    fn bosluk_ve_satir_sonu() {
        let k = kodla(&ornek(), "123456").unwrap();
        let (a, b) = k.split_at(k.len() / 2);
        assert!(coz(&format!(" \n{a}\r\n{b}  "), " 123456 ", 1_000).is_ok());
    }

    #[test]
    fn bozuk_girdiler() {
        let k = kodla(&ornek(), "123456").unwrap();
        for g in ["", "AFU2.", "AFU2.kisa", "AFU2.!!!", "merhaba", &k[5..]] {
            assert_eq!(coz(g, "123456", 1_000), Err(KodHata::Bozuk), "girdi: {g:?}");
        }
        assert_eq!(coz("AFU1.abcdef", "123456", 1_000), Err(KodHata::Surum));
        let yarim = &k[..k.len() / 2];
        assert!(coz(yarim, "123456", 1_000).is_err());
    }

    #[test]
    fn oynanmis_bayt() {
        let k = kodla(&ornek(), "123456").unwrap();
        for i in [8, k.len() / 2, k.len() - 2] {
            let mut b = k.clone().into_bytes();
            b[i] = if b[i] == b'A' { b'B' } else { b'A' };
            let r = coz(&String::from_utf8(b).unwrap(), "123456", 1_000);
            assert!(
                matches!(r, Err(KodHata::ParolaYanlis | KodHata::Bozuk)),
                "konum {i}"
            );
        }
    }

    #[test]
    fn adres_acik_metinde_gorunmez() {
        let k = kodla(&ornek(), "123456").unwrap();
        assert!(!k.contains("192.168") && !k.contains("47000"));
    }

    #[test]
    fn turkce_parola() {
        let k = kodla(&ornek(), "Şifre İğne").unwrap();
        assert!(coz(&k, "Şifre İğne", 1_000).is_ok());
        assert_eq!(coz(&k, "sifre igne", 1_000), Err(KodHata::ParolaYanlis));
    }

    #[test]
    fn rastgelelik() {
        assert_ne!(kodla(&ornek(), "1").unwrap(), kodla(&ornek(), "1").unwrap());
        assert_ne!(yeni_bilet(), yeni_bilet());
        for _ in 0..100 {
            let p = yeni_parola();
            assert!(p.len() == 6 && p.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn hata_metinleri_turkce() {
        assert_eq!(KodHata::ParolaYanlis.to_string(), "Parola yanlış.");
    }
}
