//! Google kimliğinin güvenlik sınırı. Ham JSON/e-posta yetki vermez.
//!
//! Bu modül tek başına host onayını kaldırmaz. Çağıran katman kayıt/iptal
//! durumunu güvenilir depodan almalı ve `Kabul` kararını ekran açılmadan uygulamalı.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ring::signature::{RsaPublicKeyComponents, RSA_PKCS1_2048_8192_SHA256};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{fmt, time::Duration};

/// Iroh enables ring but does not install a process default for standalone OAuth.
pub(crate) fn http_client() -> reqwest::ClientBuilder {
    let _ = rustls::crypto::ring::default_provider().install_default();
    reqwest::Client::builder()
}

pub const JWKS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";
/// Dağıtıcı gerçek istemci kimliklerini derleme yapılandırmasına koyar.
/// Yer tutucuyla giriş ve onaysız erişim mümkün değildir.
pub const CLIENT_ID_YER_TUTUCU: &str = "AFUDESK_GOOGLE_CLIENT_ID_YAPILANDIRILMADI";
const AZAMI_TOKEN: usize = 16 * 1024;
const AZAMI_JWKS: usize = 128 * 1024;

/// Sır, token veya claim içermeyen hata. UI doğrudan bu tek cümleyi kullanabilir.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("Google girişi doğrulanamadı. Yeniden giriş yap.")]
pub struct DogrulamaHatasi;

type Sonuc<T> = Result<T, DogrulamaHatasi>;

/// Yalnız imza ve claim doğrulamasıyla oluşturulur; Deserialize/alan setter yoktur.
#[derive(Clone)]
pub struct Dogrulanmis {
    sub: String,
    nonce: String,
    exp: i64,
}

impl Dogrulanmis {
    pub fn sub(&self) -> &str {
        &self.sub
    }
    pub fn nonce(&self) -> &str {
        &self.nonce
    }
    pub fn gecerlilik_sonu(&self) -> i64 {
        self.exp
    }
}

impl fmt::Debug for Dogrulanmis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Dogrulanmis([gizli])")
    }
}

#[derive(Deserialize)]
struct Baslik {
    alg: String,
    kid: String,
    #[serde(default)]
    crit: Vec<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Audience {
    Tek(String),
    Cok(Vec<String>),
}

impl Audience {
    fn degerler(&self) -> &[String] {
        match self {
            Self::Tek(a) => std::slice::from_ref(a),
            Self::Cok(a) => a,
        }
    }
}

#[derive(Deserialize)]
struct Claims {
    sub: String,
    iss: String,
    aud: Audience,
    azp: Option<String>,
    exp: i64,
    iat: i64,
    nonce: String,
}

#[derive(Deserialize)]
struct Jwk {
    kid: String,
    kty: String,
    alg: Option<String>,
    #[serde(rename = "use")]
    kullanim: Option<String>,
    n: String,
    e: String,
}

#[derive(Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

pub struct Dogrulayici {
    istemciler: Vec<String>,
    anahtarlar: Jwks,
}

/// Host ve izleyici farklı kalıcı iroh anahtarı kullanır. Google tarafından
/// imzalanan nonce ikisini birlikte bağlar; bir token başka anahtara taşınamaz.
pub fn anahtar_nonce(host: &str, izleyici: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(b"afudesk-google-v1\0");
    hash.update(host.as_bytes());
    hash.update(b"\0");
    hash.update(izleyici.as_bytes());
    URL_SAFE_NO_PAD.encode(hash.finalize())
}

impl Dogrulayici {
    /// Testler sabit fixture JWKS kullanabilir. Üretim yalnız `google_dan` ile
    /// Google'dan anahtar almalı; eşin gönderdiği JWKS hiçbir zaman kullanılmaz.
    pub fn yeni(istemciler: Vec<String>, jwks: &str) -> Sonuc<Self> {
        if istemciler.is_empty()
            || istemciler.iter().any(|s| {
                s.is_empty()
                    || s == CLIENT_ID_YER_TUTUCU
                    || !s.ends_with(".apps.googleusercontent.com")
            })
            || jwks.len() > AZAMI_JWKS
        {
            return Err(DogrulamaHatasi);
        }
        let anahtarlar: Jwks = serde_json::from_str(jwks).map_err(|_| DogrulamaHatasi)?;
        if anahtarlar.keys.is_empty() || anahtarlar.keys.len() > 64 {
            return Err(DogrulamaHatasi);
        }
        Ok(Self {
            istemciler,
            anahtarlar,
        })
    }

    /// Sabit Google HTTPS adresi, yönlendirme yok, boyut/süre sınırı var.
    /// Ağ hatası güvenlik denetimini atlatmaz; önceki key cache'iyle sınırsız
    /// çalışma yapılmaz. Çağıran katman kısa süreli cache ekleyebilir.
    pub async fn google_dan(istemciler: Vec<String>) -> Sonuc<Self> {
        let client = http_client()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|_| DogrulamaHatasi)?;
        let mut response = client
            .get(JWKS_URL)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|_| DogrulamaHatasi)?;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| DogrulamaHatasi)? {
            if bytes.len().saturating_add(chunk.len()) > AZAMI_JWKS {
                return Err(DogrulamaHatasi);
            }
            bytes.extend_from_slice(&chunk);
        }
        Self::yeni(
            istemciler,
            std::str::from_utf8(&bytes).map_err(|_| DogrulamaHatasi)?,
        )
    }

    pub fn dogrula(&self, token: &str, simdi: i64) -> Sonuc<Dogrulanmis> {
        if token.len() > AZAMI_TOKEN || simdi < 0 {
            return Err(DogrulamaHatasi);
        }
        let bolumler: Vec<_> = token.split('.').collect();
        let [h, p, s] = bolumler.as_slice() else {
            return Err(DogrulamaHatasi);
        };
        let baslik: Baslik = serde_json::from_slice(&coz(h)?).map_err(|_| DogrulamaHatasi)?;
        if baslik.alg != "RS256" || !baslik.crit.is_empty() {
            return Err(DogrulamaHatasi);
        }
        let mut uygun = self.anahtarlar.keys.iter().filter(|k| k.kid == baslik.kid);
        let key = uygun.next().ok_or(DogrulamaHatasi)?;
        if uygun.next().is_some()
            || key.kty != "RSA"
            || key.alg.as_deref().is_some_and(|a| a != "RS256")
            || key.kullanim.as_deref().is_some_and(|a| a != "sig")
        {
            return Err(DogrulamaHatasi);
        }
        let (n, e) = (coz(&key.n)?, coz(&key.e)?);
        RsaPublicKeyComponents { n: &n, e: &e }
            .verify(
                &RSA_PKCS1_2048_8192_SHA256,
                format!("{h}.{p}").as_bytes(),
                &coz(s)?,
            )
            .map_err(|_| DogrulamaHatasi)?;
        // Claim'ler yalnız imza doğrulandıktan sonra yetki için işlenir.
        let c: Claims = serde_json::from_slice(&coz(p)?).map_err(|_| DogrulamaHatasi)?;
        let aud = c.aud.degerler();
        if !matches!(
            c.iss.as_str(),
            "https://accounts.google.com" | "accounts.google.com"
        ) || c.sub.is_empty()
            || c.sub.len() > 255
            || c.sub.chars().any(char::is_control)
            || c.nonce.is_empty()
            || c.nonce.len() > 256
            || c.exp <= simdi
            || c.iat < 0
            || c.iat > simdi
            || c.exp <= c.iat
            || c.exp.saturating_sub(c.iat) > 7200
            || !aud.iter().any(|a| self.istemciler.contains(a))
            || (aud.len() > 1 && c.azp.is_none())
            || c.azp
                .as_ref()
                .is_some_and(|a| !self.istemciler.contains(a) || !aud.contains(a))
        {
            return Err(DogrulamaHatasi);
        }
        Ok(Dogrulanmis {
            sub: c.sub,
            nonce: c.nonce,
            exp: c.exp,
        })
    }
}

fn coz(s: &str) -> Sonuc<Vec<u8>> {
    URL_SAFE_NO_PAD.decode(s).map_err(|_| DogrulamaHatasi)
}

#[derive(Debug, PartialEq, Eq)]
pub enum Karar {
    Kabul,
    OnaySor,
    Red,
}

/// `kayitli`: güncel, iptal edilmemiş hesap→anahtar kaydı; salt yerel ağ
/// keşfi veya kullanıcının gönderdiği bir bool bu parametreyi belirleyemez.
/// `uzak_anahtar` yalnız QUIC el sıkışmasındaki remote_id'den alınmalıdır.
#[allow(clippy::too_many_arguments)]
pub fn karar(
    yerel: Option<&Dogrulanmis>,
    karsi: Result<&Dogrulanmis, ()>,
    host_anahtar: &str,
    izleyici_anahtar: &str,
    uzak_anahtar: &str,
    kayitli: bool,
    onaysiz: bool,
    simdi: i64,
) -> Karar {
    let Ok(karsi) = karsi else {
        return Karar::Red;
    };
    if simdi < 0
        || karsi.exp <= simdi
        || karsi.nonce != anahtar_nonce(host_anahtar, izleyici_anahtar)
        || izleyici_anahtar != uzak_anahtar
    {
        return Karar::Red;
    }
    match yerel {
        Some(y) if y.exp > simdi && y.sub == karsi.sub && kayitli && onaysiz => Karar::Kabul,
        _ => Karar::OnaySor,
    }
}

#[cfg(test)]
mod dizin_testleri {
    use super::*;
    use crate::google_dizin::Dizin;
    use iroh::SecretKey;

    fn hesap(nonce: &str, sub: &str) -> Dogrulanmis {
        Dogrulanmis {
            sub: sub.into(),
            nonce: nonce.into(),
            exp: 2000,
        }
    }

    #[test]
    fn dizin_iki_anahtari_tek_kullanimlik_google_nonce_ile_baglar() {
        let (h, v) = (SecretKey::generate(), SecretKey::generate());
        let mut d = Dizin::default();
        let nonce = d.baslat(h.public(), v.public(), 1000).unwrap();
        let a = hesap(&nonce, "hesap-a");
        let mesaj = Dizin::kayit_mesaji(&nonce, "Ev bilgisayarı");
        let hs = h.sign(&mesaj).to_bytes();
        let vs = v.sign(&mesaj).to_bytes();
        assert!(d.kaydet(&a, "Ev bilgisayarı", &hs, &vs, 1001).is_ok());
        assert_eq!(d.liste(&a, 1001).unwrap().len(), 1);
        assert!(d.liste(&hesap(&nonce, "hesap-b"), 1001).unwrap().is_empty());
        assert!(d.kaydet(&a, "Ev bilgisayarı", &hs, &vs, 1002).is_err());
        assert!(d.kayitli("hesap-a", h.public(), v.public()));
        d.kaldir("hesap-a", h.public());
        assert!(!d.kayitli("hesap-a", h.public(), v.public()));
        assert!(d.kaydet(&a, "Ev bilgisayarı", &hs, &vs, 1003).is_err());
    }

    #[test]
    fn dizin_sahte_anahtar_ad_degisikligi_ve_gecikmis_kaniti_reddeder() {
        let (h, v, sahte) = (
            SecretKey::generate(),
            SecretKey::generate(),
            SecretKey::generate(),
        );
        for tur in 0..4 {
            let mut d = Dizin::default();
            let nonce = d.baslat(h.public(), v.public(), 1000).unwrap();
            let a = hesap(&nonce, "hesap-a");
            let mesaj = Dizin::kayit_mesaji(&nonce, "Ev");
            let hs = h.sign(&mesaj).to_bytes();
            let vs = if tur == 0 {
                sahte.sign(&mesaj)
            } else {
                v.sign(&mesaj)
            }
            .to_bytes();
            let ad = if tur == 1 { "Başka" } else { "Ev" };
            let simdi = if tur == 2 {
                1301
            } else if tur == 3 {
                999
            } else {
                1001
            };
            assert!(d.kaydet(&a, ad, &hs, &vs, simdi).is_err());
            assert!(!d.kayitli("hesap-a", h.public(), v.public()));
        }
    }

    #[test]
    fn dizin_baska_hesap_ayni_anahtarlari_sahiplenemez() {
        let (h, v) = (SecretKey::generate(), SecretKey::generate());
        let mut d = Dizin::default();
        for sub in ["hesap-a", "hesap-b"] {
            let nonce = d.baslat(h.public(), v.public(), 1000).unwrap();
            let mesaj = Dizin::kayit_mesaji(&nonce, "Ev");
            let r = d.kaydet(
                &hesap(&nonce, sub),
                "Ev",
                &h.sign(&mesaj).to_bytes(),
                &v.sign(&mesaj).to_bytes(),
                1001,
            );
            assert_eq!(r.is_ok(), sub == "hesap-a");
        }
    }

    #[test]
    fn dizin_ayri_cihazlar_arasi_ayni_hesabi_ve_kaldirmayi_kontrol_eder() {
        let (h1, v1, h2, v2) = (
            SecretKey::generate(),
            SecretKey::generate(),
            SecretKey::generate(),
            SecretKey::generate(),
        );
        let mut d = Dizin::default();
        for (h, v) in [(&h1, &v1), (&h2, &v2)] {
            let nonce = d.baslat(h.public(), v.public(), 1000).unwrap();
            let mesaj = Dizin::kayit_mesaji(&nonce, "Cihaz");
            d.kaydet(
                &hesap(&nonce, "hesap-a"),
                "Cihaz",
                &h.sign(&mesaj).to_bytes(),
                &v.sign(&mesaj).to_bytes(),
                1001,
            )
            .unwrap();
        }
        assert!(d.kayitli("hesap-a", h1.public(), v2.public()));
        assert!(!d.kayitli("hesap-b", h1.public(), v2.public()));
        d.kaldir("hesap-b", h2.public());
        assert!(d.kayitli("hesap-a", h1.public(), v2.public()));
        d.kaldir("hesap-a", h2.public());
        assert!(!d.kayitli("hesap-a", h1.public(), v2.public()));
    }
}
