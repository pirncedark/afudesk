//! Google kimliğinin güvenlik sınırı. Ham JSON/e-posta yetki vermez.
//!
//! Bu modül tek başına host onayını kaldırmaz. Çağıran katman kayıt/iptal
//! durumunu güvenilir depodan almalı ve `Kabul` kararını ekran açılmadan uygulamalı.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ring::signature::{RsaPublicKeyComponents, RSA_PKCS1_2048_8192_SHA256};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{fmt, time::Duration};

/// Android has no platform-verifier JNI context in this bridge. Use Mozilla's
/// roots and an explicit provider instead; certificate/hostname checks stay on.
#[cfg(any(target_os = "android", test))]
fn android_http_client() -> Sonuc<reqwest::ClientBuilder> {
    let roots = rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder_with_provider(std::sync::Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|_| DogrulamaHatasi)?
    .with_root_certificates(roots)
    .with_no_client_auth();
    Ok(reqwest::Client::builder().tls_backend_preconfigured(config))
}

pub(crate) fn http_client() -> Sonuc<reqwest::ClientBuilder> {
    #[cfg(target_os = "android")]
    {
        android_http_client()
    }
    #[cfg(not(target_os = "android"))]
    {
        // Preserve native certificate verification on desktop.
        let _ = rustls::crypto::ring::default_provider().install_default();
        Ok(reqwest::Client::builder())
    }
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
pub struct DogrulamaHatasi {
    neden: &'static str,
}

// Preserve the existing credential-free default error at other call sites.
#[allow(non_upper_case_globals)]
pub const DogrulamaHatasi: DogrulamaHatasi = DogrulamaHatasi { neden: "diger" };

impl DogrulamaHatasi {
    pub const NONCE: Self = Self { neden: "nonce" };

    pub fn nedeni(&self) -> &'static str {
        self.neden
    }

    fn nedenli(neden: &'static str) -> Self {
        Self { neden }
    }
}

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

/// Only well-formed Google client IDs can establish a shared project.
fn istemci_projesi(id: &str) -> Option<&str> {
    let (proje, istemci) = id
        .strip_suffix(".apps.googleusercontent.com")?
        .split_once('-')?;
    if proje.is_empty()
        || !proje.bytes().all(|b| b.is_ascii_digit())
        || istemci.is_empty()
        || !istemci
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return None;
    }
    Some(proje)
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
        let client = http_client()?
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
            return Err(DogrulamaHatasi::nedenli("imza"));
        }
        let bolumler: Vec<_> = token.split('.').collect();
        let [h, p, s] = bolumler.as_slice() else {
            return Err(DogrulamaHatasi::nedenli("imza"));
        };
        let baslik: Baslik =
            serde_json::from_slice(&coz(h)?).map_err(|_| DogrulamaHatasi::nedenli("imza"))?;
        if baslik.alg != "RS256" || !baslik.crit.is_empty() {
            return Err(DogrulamaHatasi::nedenli("imza"));
        }
        let mut uygun = self.anahtarlar.keys.iter().filter(|k| k.kid == baslik.kid);
        let key = uygun.next().ok_or(DogrulamaHatasi::nedenli("imza"))?;
        if uygun.next().is_some()
            || key.kty != "RSA"
            || key.alg.as_deref().is_some_and(|a| a != "RS256")
            || key.kullanim.as_deref().is_some_and(|a| a != "sig")
        {
            return Err(DogrulamaHatasi::nedenli("imza"));
        }
        let (n, e) = (coz(&key.n)?, coz(&key.e)?);
        RsaPublicKeyComponents { n: &n, e: &e }
            .verify(
                &RSA_PKCS1_2048_8192_SHA256,
                format!("{h}.{p}").as_bytes(),
                &coz(s)?,
            )
            .map_err(|_| DogrulamaHatasi::nedenli("imza"))?;
        // Claim'ler yalnız imza doğrulandıktan sonra yetki için işlenir.
        let c: Claims =
            serde_json::from_slice(&coz(p)?).map_err(|_| DogrulamaHatasi::nedenli("claim"))?;
        let aud = c.aud.degerler();
        let neden = if !matches!(
            c.iss.as_str(),
            "https://accounts.google.com" | "accounts.google.com"
        ) {
            Some("iss")
        } else if c.sub.is_empty() || c.sub.len() > 255 || c.sub.chars().any(char::is_control) {
            Some("sub")
        } else if c.nonce.is_empty() || c.nonce.len() > 256 {
            Some("nonce")
        } else if c.exp <= simdi {
            Some("exp")
        } else if c.iat < 0 || c.iat > simdi.saturating_add(300) {
            Some("iat")
        } else if c.exp <= c.iat || c.exp.saturating_sub(c.iat) > 7200 {
            Some("exp")
        } else if !aud.iter().any(|a| self.istemciler.contains(a)) {
            Some("aud")
        } else if (aud.len() > 1 && c.azp.is_none())
            || c.azp.as_ref().is_some_and(|azp| {
                !self.istemciler.contains(azp)
                    && !istemci_projesi(azp).is_some_and(|proje| {
                        aud.iter().any(|a| {
                            self.istemciler.contains(a) && istemci_projesi(a) == Some(proje)
                        })
                    })
            })
        {
            Some("azp")
        } else {
            None
        };
        if let Some(neden) = neden {
            return Err(DogrulamaHatasi::nedenli(neden));
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

    #[tokio::test]
    async fn android_tls_istemcisi_jni_baslatmadan_kurulur() {
        // Exercise the Android configuration on desktop too. The Android test
        // target additionally catches accidental use of the JNI verifier.
        let client = android_http_client().unwrap().build().unwrap();
        assert_eq!(
            client.get(JWKS_URL).build().unwrap().url().scheme(),
            "https"
        );
    }

    // Ephemeral test-only RSA key; no production credentials.
    const TEST_KEY: &str = "MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDr7-Tg9GQeazhH-qgLsgHQ-wg-KAoI1Azfs28NfaqqwrYWcMBMjFxQnKexWbynqtN-MSlu_bVazyvGT7c04A20dbOM-SElVQr3Y_tILWmxGi1xXKq1QYHfIaMBEeBVeqYbJpsk74T5LbQtAa-75p2R-DIcD5XswkIflnY6tQuB1Lb6tGPa3X-DusHhff_7Ei1B8zBazt3L6hBLXW21WmmH5wR-aHVBsb8F31TpJ2gbDEu5KUDbjOBIuACDzTnSmMov2E97mfKwbCl4HaUVakwX17U6ZfCIqTQxffy8sqXxkAfIxukAI74GjZIFmf55v4PqvM2LfnZrkMO2vPZ0GuBLAgMBAAECggEALL3VXYN66PnUNnImI_GhvoucSI03Oe1cc5-HU7uc_bzm4uKG1P1ZFEBg3JTz3afba3bPMKkUFyBqYLUFPRWwA6S-Y-Fqk7XI2pkvMcTK4jbgnzq_dJfzlOjADJfjPtJ_ZW6wxRRW1xn_rPlA5hUZlHDIr-GUwM0l1O-nhAKJKBldUIwT6HTkdo79_Ufpik6qdjm3pFrOtJKmwqnUHbxsEaVXRPeA-ZNiujddfECYG1sH7U_pzf8th0DDh_TpLWUoQ3WGmBZefsvrI-pY-xzwLfTKE4RTQZksJOeRfjDRtzqEsKyBSQ_lxAtR2JSa4jzHe548f5JZemp-N3gGVHkvoQKBgQD5tSSYuoDfed4Oc27waTO5pqnog3XeWJihfEcQMd0IQIhred5lVancxYXmOwd9LCpQYrMwRo1lcmfB2uRVFPatt5ITDHAhtsc6Ab-hR20QYfSzjd10PuM5auKeqMjGwJ5lCYkd-zJPrZ2nFoTpd-yDmse5RrnkCFPmy0Y7HZm1ZQKBgQDx4er-scWTsM4F-9Ic9FOEue7wDFv7wWZ1CzkpM7iAi891n3dmqUijw2VivaeIJJ8sr15FW4x3fcZCRTQDl1qkFCDJYM6NYzlz9M_yuAhe_4cNsn-ctkc4h2qKXq4jP4XdgtJPZSfV6ypL6haTgbUjkaw9bb3gUTGk7vXpd_d77wKBgQCVW-Qn9mZfRWWs_x68bjCiOy4mBXvbIuMotE1NQoYTCYaPwrVjp3mStBMDQwuXs3Mb4e2MUqcApw2oxmUM_x0Ni1-0RRxNDibca29T6IUz52OWgqSTg4omLeifPhXZzBzyEmIRrM7LGe3So4WcoD63-Ty9dbB61QBaa_v2IB5wgQKBgHa-TqvcP79iY7HCTwLR2xJkaO1FU_7CkfbRnK0ia-gHOsBmmao3XqaubN5Ndqo6OD4qlV_9MncSsrqj2IYOznuQOgxCTwRP-st3GoLwX4Orc_SAdW9MeKhCqkXldx1suN3mtDIv2Z-FxAzijVS6r18Xy3OYpUcy9OkgOm7z6vXXAoGBAJ1RaTMv5B87GqkeJwXgoLsJXFnQ_tKrk93ZvbXMK5xMLQ0mcMJ6Z9GVW5oiPKZ_BTTC4uCzBgOmTJcF15ocQEZqkkZc6gCf--06LELJjxuA9bJmEZbt5Sz23IKgrr5nMKKyRIRQ9U9uFPu3p7ixGYG62D0e6WykqOWCCT1sASAi";
    const TEST_JWKS: &str = r#"{"keys": [{"kid": "android-test", "kty": "RSA", "alg": "RS256", "n": "6-_k4PRkHms4R_qoC7IB0PsIPigKCNQM37NvDX2qqsK2FnDATIxcUJynsVm8p6rTfjEpbv21Ws8rxk-3NOANtHWzjPkhJVUK92P7SC1psRotcVyqtUGB3yGjARHgVXqmGyabJO-E-S20LQGvu-adkfgyHA-V7MJCH5Z2OrULgdS2-rRj2t1_g7rB4X3_-xItQfMwWs7dy-oQS11ttVpph-cEfmh1QbG_Bd9U6SdoGwxLuSlA24zgSLgAg8050pjKL9hPe5nysGwpeB2lFWpMF9e1OmXwiKk0MX38vLKl8ZAHyMbpACO-Bo2SBZn-eb-D6rzNi352a5DDtrz2dBrgSw", "e": "AQAB"}]}"#;
    const WEB: &str = "319814222024-web.apps.googleusercontent.com";
    const ANDROID: &str = "319814222024-android.apps.googleusercontent.com";
    const NOW: i64 = 1_700_000_100;

    fn signed_claims(change: impl FnOnce(&mut serde_json::Value)) -> String {
        use ring::{
            rand::SystemRandom,
            signature::{RsaKeyPair, RSA_PKCS1_SHA256},
        };
        let key = RsaKeyPair::from_pkcs8(&coz(TEST_KEY).unwrap()).unwrap();
        let mut claims = serde_json::json!({
            "sub":"test-account", "iss":"https://accounts.google.com",
            "aud":WEB, "azp":ANDROID, "iat":NOW, "exp":NOW+3600, "nonce":"test-nonce"
        });
        change(&mut claims);
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256","kid":"android-test"}"#);
        let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap());
        let input = format!("{header}.{payload}");
        let mut signature = vec![0; key.public().modulus_len()];
        key.sign(
            &RSA_PKCS1_SHA256,
            &SystemRandom::new(),
            input.as_bytes(),
            &mut signature,
        )
        .unwrap();
        format!("{input}.{}", URL_SAFE_NO_PAD.encode(signature))
    }

    #[test]
    fn android_azp_ayni_projeden_kabul_edilir() {
        let d = Dogrulayici::yeni(vec![WEB.into()], TEST_JWKS).unwrap();
        assert!(d.dogrula(&signed_claims(|_| {}), NOW).is_ok());
    }

    #[test]
    fn android_azp_yabanci_ve_bozuk_projeden_reddedilir() {
        let d = Dogrulayici::yeni(vec![WEB.into()], TEST_JWKS).unwrap();
        for azp in [
            "999-android.apps.googleusercontent.com",
            "319814222024-.apps.googleusercontent.com",
            "319814222024-android.evil",
            "319814222024x-android.apps.googleusercontent.com",
        ] {
            assert!(
                d.dogrula(&signed_claims(|c| c["azp"] = azp.into()), NOW)
                    .is_err(),
                "{azp}"
            );
        }
    }

    #[test]
    fn android_iat_300_saniye_toleransi() {
        let d = Dogrulayici::yeni(vec![WEB.into()], TEST_JWKS).unwrap();
        for offset in [120, 300, 301, 600] {
            let token = signed_claims(|c| {
                c["azp"] = WEB.into();
                c["iat"] = (NOW + offset).into();
            });
            assert_eq!(d.dogrula(&token, NOW).is_ok(), offset <= 300, "{offset}");
        }
    }

    #[test]
    fn android_aud_ve_coklu_aud_azp_zorunlulugu_korunur() {
        let d = Dogrulayici::yeni(vec![WEB.into()], TEST_JWKS).unwrap();
        assert!(d
            .dogrula(&signed_claims(|c| c["aud"] = ANDROID.into()), NOW)
            .is_err());
        assert!(d
            .dogrula(
                &signed_claims(|c| {
                    c["aud"] = serde_json::json!([WEB, ANDROID]);
                    c.as_object_mut().unwrap().remove("azp");
                }),
                NOW
            )
            .is_err());
        assert!(d
            .dogrula(
                &signed_claims(|c| {
                    c.as_object_mut().unwrap().remove("azp");
                }),
                NOW
            )
            .is_ok());
    }

    #[test]
    fn android_izinli_azp_aud_disinda_da_kabul_edilir() {
        let other = "999-desktop.apps.googleusercontent.com";
        let d = Dogrulayici::yeni(vec![WEB.into(), other.into()], TEST_JWKS).unwrap();
        assert!(d
            .dogrula(&signed_claims(|c| c["azp"] = other.into()), NOW)
            .is_ok());
    }

    #[test]
    fn android_hata_nedeni_claim_degerlerini_aciga_cikarmaz() {
        let d = Dogrulayici::yeni(vec![WEB.into()], TEST_JWKS).unwrap();
        for (field, value, reason) in [
            ("aud", serde_json::json!(ANDROID), "aud"),
            (
                "azp",
                serde_json::json!("999-other.apps.googleusercontent.com"),
                "azp",
            ),
            ("iat", serde_json::json!(NOW + 600), "iat"),
            ("nonce", serde_json::json!(""), "nonce"),
        ] {
            let e = d
                .dogrula(&signed_claims(|c| c[field] = value), NOW)
                .unwrap_err();
            let text = format!("{e:?} {e}");
            assert!(text.contains(reason), "missing {reason}: {text}");
            for secret in [WEB, ANDROID, "test-account", "test-nonce"] {
                assert!(!text.contains(secret));
            }
        }
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/google.json")).unwrap();
        let e = d
            .dogrula(fixture["bad_signature"].as_str().unwrap(), NOW)
            .unwrap_err();
        assert!(format!("{e:?}").contains("imza"));
    }

    #[test]
    fn android_exp_icin_tolerans_yok() {
        let d = Dogrulayici::yeni(vec![WEB.into()], TEST_JWKS).unwrap();
        assert!(d
            .dogrula(
                &signed_claims(|c| {
                    c["iat"] = (NOW - 3600).into();
                    c["exp"] = NOW.into();
                }),
                NOW
            )
            .is_err());
    }

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
