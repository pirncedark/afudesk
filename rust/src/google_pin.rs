//! PIN doğrulaması ve şifreli yerel deneme/hatırlama kaydı.
//! Tek başına uzaktan erişim açmaz. Ham PIN veya hatırlama sırrı saklanmaz.
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use std::fmt;

pub struct Pin(String);

impl Pin {
    pub fn olustur(pin: &str) -> Result<Self, &'static str> {
        if !(6..=64).contains(&pin.len()) || !pin.bytes().all(|b| b.is_ascii_digit()) {
            return Err("PIN en az 6 rakam olmalı. Yeniden dene.");
        }
        let tuz = SaltString::generate(&mut rand::rngs::OsRng);
        let hash = Argon2::default()
            .hash_password(pin.as_bytes(), &tuz)
            .map_err(|_| "PIN kaydedilemedi. Yeniden dene.")?;
        Ok(Self(hash.to_string()))
    }

    pub fn phc(&self) -> &str {
        &self.0
    }

    /// Yalnız bu uygulamanın ürettiği sabit maliyetli Argon2id PHC kabul edilir.
    /// Bozuk kaydın aşırı bellek/CPU parametreleri çalıştırılmaz.
    pub fn phc_yukle(phc: &str) -> Result<Self, &'static str> {
        let hash = PasswordHash::new(phc).map_err(|_| "PIN kaydı bozuk. Yeniden kur.")?;
        if phc.len() > 256
            || hash.algorithm.as_str() != "argon2id"
            || hash.version != Some(19)
            || hash.params.iter().count() != 3
            || hash.params.get_decimal("m") != Some(19456)
            || hash.params.get_decimal("t") != Some(2)
            || hash.params.get_decimal("p") != Some(1)
            || hash.salt.is_none_or(|s| s.len() != 22)
            || hash.hash.is_none_or(|h| h.len() != 32)
        {
            return Err("PIN kaydı bozuk. Yeniden kur.");
        }
        Ok(Self(phc.into()))
    }

    pub fn dogrula(&self, pin: &str) -> bool {
        if !(6..=64).contains(&pin.len()) || !pin.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
        PasswordHash::new(&self.0).is_ok_and(|h| {
            Argon2::default()
                .verify_password(pin.as_bytes(), &h)
                .is_ok()
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PinSonucu {
    Kabul,
    Gerekli,
    Yanlis,
    Kilitli,
}

/// Her işlem dosya kilidi altında güncel DPAPI kaydını okur ve sonucu vermeden
/// önce atomik yazar. Aynı host'un tüm bağlantıları tek deneme bütçesini paylaşır.
/// `sub`, `host` ve `izleyici` yalnız doğrulanmış Google/dizin/iroh katmanından
/// gelmelidir. Hatırlama bu katmanların güncel üyelik kontrolünün yerine geçmez.
/// Android adaptörü henüz yoktur; desteklenmeyen platformda erişim açılmaz.
pub struct PinKasasi;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PinKaydi {
    surum: u8,
    sub: String,
    host: String,
    phc: String,
    hata: u8,
    kilit_sonu: i64,
    son_zaman: i64,
    hatirlanan: std::collections::BTreeMap<String, i64>,
}

impl PinKasasi {
    fn hata() -> std::io::Error {
        std::io::Error::other("PIN kaydı açılamadı. Yeniden giriş yap.")
    }

    fn anahtar(key: &str) -> bool {
        key.parse::<iroh::EndpointId>().is_ok()
    }

    fn kilit(yol: &std::path::Path) -> std::io::Result<std::fs::File> {
        // Sabit ayrı dosya: asıl kaydın atomik yenilenmesi kilidi düşürmez.
        // Bu dosya silinmez; süreç bitince işletim sistemi kilidi bırakır.
        let mut ad = yol.as_os_str().to_os_string();
        ad.push(".lock");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(ad)?;
        file.lock()?;
        Ok(file)
    }

    fn oku(yol: &std::path::Path, sub: &str, host: &str) -> std::io::Result<PinKaydi> {
        let k: PinKaydi =
            serde_json::from_slice(&crate::google_depo::oku(yol)?).map_err(|_| Self::hata())?;
        if k.surum != 1
            || k.sub != sub
            || k.host != host
            || !Self::anahtar(host)
            || sub.is_empty()
            || k.hata > 5
            || k.kilit_sonu < 0
            || k.son_zaman < 0
            || k.hatirlanan.len() > 100
            || k.hatirlanan
                .iter()
                .any(|(key, son)| !Self::anahtar(key) || *son < 0)
        {
            return Err(Self::hata());
        }
        Pin::phc_yukle(&k.phc).map_err(|_| Self::hata())?;
        Ok(k)
    }

    fn yaz(yol: &std::path::Path, k: &PinKaydi) -> std::io::Result<()> {
        crate::google_depo::yaz(yol, &serde_json::to_vec(k).map_err(|_| Self::hata())?)
    }

    /// Açık ilk kurulum; mevcut kayıt sessizce değiştirilmez.
    pub fn kur(yol: &std::path::Path, sub: &str, host: &str, pin: &str) -> std::io::Result<()> {
        let _kilit = Self::kilit(yol)?;
        if yol.try_exists()? || sub.is_empty() || sub.len() > 255 || !Self::anahtar(host) {
            return Err(Self::hata());
        }
        let pin = Pin::olustur(pin).map_err(|_| Self::hata())?;
        Self::yaz(
            yol,
            &PinKaydi {
                surum: 1,
                sub: sub.into(),
                host: host.into(),
                phc: pin.phc().into(),
                hata: 0,
                kilit_sonu: 0,
                son_zaman: 0,
                hatirlanan: Default::default(),
            },
        )
    }

    /// Beş yanlış deneme host'u 300 saniye kilitler. Hatırlama süresi açıkça
    /// verilir (0: kapalı, üst sınır 30 gün); varsayılan ürün kararı verilmez.
    /// `simdi` güvenilir host saatidir, istemciden alınmaz. Saat geri giderse red.
    #[allow(clippy::too_many_arguments)]
    pub fn dene(
        yol: &std::path::Path,
        sub: &str,
        host: &str,
        izleyici: &str,
        pin: Option<&str>,
        hatirla_sn: u64,
        simdi: i64,
    ) -> std::io::Result<PinSonucu> {
        let _kilit = Self::kilit(yol)?;
        let mut k = Self::oku(yol, sub, host)?;
        if simdi < k.son_zaman || !Self::anahtar(izleyici) || hatirla_sn > 30 * 86400 {
            return Err(Self::hata());
        }
        k.son_zaman = simdi;
        k.hatirlanan.retain(|_, son| *son > simdi);
        let sonuc = if k.kilit_sonu > simdi {
            PinSonucu::Kilitli
        } else {
            if k.kilit_sonu != 0 {
                k.hata = 0;
                k.kilit_sonu = 0;
            }
            if pin.is_none() && k.hatirlanan.contains_key(izleyici) {
                PinSonucu::Kabul
            } else if let Some(pin) = pin {
                if Pin::phc_yukle(&k.phc)
                    .map_err(|_| Self::hata())?
                    .dogrula(pin)
                {
                    k.hata = 0;
                    if hatirla_sn > 0 {
                        if k.hatirlanan.len() >= 100 && !k.hatirlanan.contains_key(izleyici) {
                            return Err(Self::hata());
                        }
                        k.hatirlanan.insert(
                            izleyici.into(),
                            simdi
                                .checked_add(hatirla_sn as i64)
                                .ok_or_else(Self::hata)?,
                        );
                    }
                    PinSonucu::Kabul
                } else {
                    k.hata += 1;
                    if k.hata >= 5 {
                        k.kilit_sonu = simdi.checked_add(300).ok_or_else(Self::hata)?;
                        PinSonucu::Kilitli
                    } else {
                        PinSonucu::Yanlis
                    }
                }
            } else {
                PinSonucu::Gerekli
            }
        };
        Self::yaz(yol, &k)?;
        Ok(sonuc)
    }

    /// Cihaz kaldırma sırasında çağrılır; eski hatırlama yetkisi iptal edilir.
    pub fn unut(
        yol: &std::path::Path,
        sub: &str,
        host: &str,
        izleyici: &str,
    ) -> std::io::Result<()> {
        let _kilit = Self::kilit(yol)?;
        let mut k = Self::oku(yol, sub, host)?;
        k.hatirlanan.remove(izleyici);
        Self::yaz(yol, &k)
    }

    /// Hesaptan çıkışta PIN ve tüm hatırlamalar silinir. Silme hatası saklanmaz.
    pub fn cikis(yol: &std::path::Path) -> std::io::Result<()> {
        let _kilit = Self::kilit(yol)?;
        crate::google_depo::sil(yol)
    }
}

impl fmt::Debug for Pin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Pin([gizli])")
    }
}
