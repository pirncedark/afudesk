//! Account operations contain no printable credentials. JSON is only a small
//! bridge DTO; authorization always stays inside the core.
use super::afudesk;
use afudesk_core::{
    google_baglanti::{self, Erisim},
    google_drive::CihazKaydi,
    google_oauth::{Giris, MasaustuGiris},
    kimlik,
};
use anyhow::{anyhow, ensure, Context, Result};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex, Once};

// Only fixed diagnostic messages are persisted. Panic payloads can contain
// request URLs or credentials, so unknown payloads must never be copied.
fn guvenli_panik_mesaji(mesaj: &str) -> &'static str {
    if mesaj.contains("Expect rustls-platform-verifier to be initialized") {
        "rustls-platform-verifier: Android JNI context is not initialized"
    } else if mesaj.contains("CryptoProvider") {
        "rustls: CryptoProvider is not configured"
    } else {
        "Rust panic (payload omitted)"
    }
}

fn panik_tanisini_kur() {
    static KURULDU: Once = Once::new();
    let Some(klasor) = afudesk::veri() else {
        return;
    };
    KURULDU.call_once(|| {
        let onceki = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |bilgi| {
            use std::io::Write;
            let mesaj = bilgi
                .payload()
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| bilgi.payload().downcast_ref::<String>().map(String::as_str))
                .unwrap_or("");
            // Capture the directory before installing the hook: no application
            // mutex may be locked while unwinding another application mutex.
            let _ = std::fs::create_dir_all(&klasor);
            if let Ok(mut dosya) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(klasor.join("google_panic.log"))
            {
                let _ = write!(
                    dosya,
                    "{} | {}",
                    afudesk_core::kod::simdi(),
                    guvenli_panik_mesaji(mesaj)
                );
                if let Some(konum) = bilgi.location() {
                    let _ = write!(dosya, " | {}:{}", konum.file(), konum.line());
                }
                let _ = writeln!(dosya);
            }
            onceki(bilgi);
        }));
    });
}

enum Bekleyen {
    Masaustu(MasaustuGiris, String),
    Android(Giris),
}
// Keep the audience allowlist with the transaction until remote access starts.
static BEKLEYEN: Mutex<Option<(Bekleyen, Vec<String>)>> = Mutex::new(None);
static HESAP: Mutex<Option<Arc<Erisim>>> = Mutex::new(None);
// Serializes sign-in, sign-out and directory mutations across await points.
static ISLEM: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn masaustu_sir(derlenen: Option<&str>) -> Result<&str> {
    derlenen
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| anyhow!("G2"))
}

// Android binaries never embed the desktop credential, even if the build
// machine happens to have the environment variable set.
#[cfg(not(target_os = "android"))]
const DESKTOP_SECRET: Option<&str> = option_env!("AFUDESK_GOOGLE_DESKTOP_CLIENT_SECRET");
#[cfg(target_os = "android")]
const DESKTOP_SECRET: Option<&str> = None;

fn web_istemci<'a>(ayarlar: &'a Value, derlenen: Option<&'a str>) -> Option<&'a str> {
    ayarlar["webClientId"]
        .as_str()
        .filter(|id| !id.is_empty())
        .or(derlenen)
}

fn istemciler(desktop: Option<&str>, web: Option<&str>) -> Vec<String> {
    [desktop, web]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}
fn cihaz() -> Result<CihazKaydi> {
    let v = afudesk::veri().ok_or_else(|| anyhow!("Cihaz kaydedilemedi, yeniden dene."))?;
    Ok(CihazKaydi {
        v: 1,
        sub: String::new(),
        host: kimlik::yukle_veya_uret(&v.join(kimlik::HOST_DOSYASI))?
            .kimlik()
            .to_string(),
        izleyici: kimlik::yukle_veya_uret(&v.join(kimlik::IZLEYICI_DOSYASI))?
            .kimlik()
            .to_string(),
        ad: afudesk::cihaz_adi(),
        platform: std::env::consts::OS.into(),
        adresler: vec![],
        relaylar: vec![],
        son_gorulme: afudesk_core::kod::simdi(),
        erisim: true,
    })
}
fn hesap() -> Result<Arc<Erisim>> {
    HESAP
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| anyhow!("Hesabın kapalı, yeniden giriş yap."))
}
fn adresleri_yenile(k: &mut CihazKaydi) {
    if let Some((adresler, relaylar)) = afudesk::host_adresleri() {
        k.adresler = adresler;
        k.relaylar = relaylar;
    }
    k.son_gorulme = afudesk_core::kod::simdi();
}

// Emit only fixed stage/rule names; never copy arbitrary error-chain text.
fn guvenli_islem_hatasi(e: anyhow::Error) -> anyhow::Error {
    let code = ["G2", "G3", "G4", "G5"]
        .into_iter()
        .find(|code| e.chain().any(|cause| cause.to_string() == *code))
        .unwrap_or("G9");
    if code == "G2" {
        anyhow!("Google girişi bu sürümde yapılandırılmamış (G2).")
    } else if code == "G4" {
        let neden = e
            .chain()
            .find_map(|cause| cause.downcast_ref::<afudesk_core::google::DogrulamaHatasi>())
            .map_or("diger", |hata| hata.nedeni());
        anyhow!("Google işlemi tamamlanamadı (G4:{neden}), yeniden giriş yap.")
    } else {
        anyhow!("Google işlemi tamamlanamadı ({code}), yeniden giriş yap.")
    }
}

/// No tokens are returned except the native sign-in nonce. Only fixed stage
/// and validation-rule names can leave this boundary.
pub async fn google_islem(islem: String, veri: String) -> Result<String> {
    panik_tanisini_kur();
    islem_yap(&islem, &veri)
        .await
        .map(|v| v.to_string())
        .map_err(guvenli_islem_hatasi)
}

async fn islem_yap(islem: &str, veri: &str) -> Result<Value> {
    let _islem = ISLEM.lock().await;
    match islem {
        "baslat" => {
            ensure!(HESAP.lock().unwrap().is_none(), "already signed in");
            ensure!(BEKLEYEN.lock().unwrap().is_none(), "sign-in pending");
            // Dart defines are not Rust compiler environment variables. Accept
            // the desktop ID explicitly from the trusted Flutter application.
            let ayarlar: Value = if veri.is_empty() {
                json!({})
            } else {
                serde_json::from_str(veri)?
            };
            let desktop = ayarlar["desktopClientId"]
                .as_str()
                .filter(|id| !id.is_empty())
                .or(option_env!("AFUDESK_GOOGLE_DESKTOP_CLIENT_ID"));
            // Same trusted BuildConfig value used by Credential Manager. This
            // also works when Gradle is invoked with -P instead of an env var.
            let web = web_istemci(&ayarlar, option_env!("AFUDESK_GOOGLE_WEB_CLIENT_ID"));
            let kabul_edilen = istemciler(desktop, web);
            let secret = if cfg!(target_os = "android") {
                None
            } else {
                Some(masaustu_sir(DESKTOP_SECRET)?)
            };
            let k = cihaz()?;
            if cfg!(target_os = "android") {
                let id = web.ok_or_else(|| anyhow!("G2"))?;
                let g = Giris::yeni(id, 1, &k.host, &k.izleyici).context("G2")?;
                let nonce = g.nonce().to_owned();
                *BEKLEYEN.lock().unwrap() = Some((Bekleyen::Android(g), kabul_edilen));
                Ok(json!({"nonce":nonce}))
            } else {
                let id = desktop.ok_or_else(|| anyhow!("G2"))?;
                let g = MasaustuGiris::baslat(id, &k.host, &k.izleyici).await?;
                let url = g.url().to_owned();
                *BEKLEYEN.lock().unwrap() = Some((
                    Bekleyen::Masaustu(g, secret.ok_or_else(|| anyhow!("G2"))?.to_owned()),
                    kabul_edilen,
                ));
                Ok(json!({"url":url}))
            }
        }
        "iptal" => {
            BEKLEYEN.lock().unwrap().take();
            Ok(json!({}))
        }
        "tamamla" => {
            let (pending, kabul_edilen) = BEKLEYEN
                .lock()
                .unwrap()
                .take()
                .ok_or_else(|| anyhow!("no transaction"))?;
            let (id, access) = match pending {
                Bekleyen::Masaustu(g, secret) => {
                    let token = g.kod_bekle().await?.oturum(Some(&secret)).await?;
                    (token.id_token, token.access_token)
                }
                Bekleyen::Android(g) => {
                    let v: Value = serde_json::from_str(veri)?;
                    let id = v["idToken"].as_str().ok_or_else(|| anyhow!("id"))?;
                    let d = afudesk_core::google::Dogrulayici::google_dan(kabul_edilen.clone())
                        .await
                        .context("G5")?;
                    let identity = d.dogrula(id, afudesk_core::kod::simdi()).context("G4")?;
                    if identity.nonce() != g.nonce() {
                        return Err(
                            anyhow!(afudesk_core::google::DogrulamaHatasi::NONCE).context("G4")
                        );
                    }
                    (
                        id.to_owned(),
                        v["accessToken"]
                            .as_str()
                            .ok_or_else(|| anyhow!("access"))?
                            .to_owned(),
                    )
                }
            };
            let mut k = cihaz()?;
            adresleri_yenile(&mut k);
            let g = Erisim::yeni(id, access, kabul_edilen, k)
                .await
                .context("G4")?;
            g.dizin.hesabi_dogrula().await.context("G3")?;
            g.dizin.kaydet(&g.cihaz).await.context("G3")?;
            google_baglanti::etkinlestir(g.clone());
            *HESAP.lock().unwrap() = Some(g);
            Ok(json!({"ad":"Google hesabın"}))
        }
        "liste" => {
            let g = hesap()?;
            ensure!(
                g.hesap.gecerlilik_sonu() > afudesk_core::kod::simdi(),
                "expired"
            );
            let list = g.dizin.liste().await?;
            Ok(json!(list.into_iter().filter(|k| k.host != g.cihaz.host && k.platform != "android")
                .map(|k| json!({"id":k.host,"ad":k.ad,"cevrimici":k.erisim && k.guncel(g.hesap.sub(),afudesk_core::kod::simdi())})).collect::<Vec<_>>()))
        }
        "yenile" => {
            let g = hesap()?;
            ensure!(
                g.hesap.gecerlilik_sonu() > afudesk_core::kod::simdi(),
                "expired"
            );
            let mut k = g.cihaz.clone();
            adresleri_yenile(&mut k);
            g.dizin.yenile(&k).await?;
            Ok(json!({}))
        }
        "cikar" => {
            let g = hesap()?;
            let list = g.dizin.liste().await?;
            let k = list
                .iter()
                .find(|k| k.host == veri)
                .ok_or_else(|| anyhow!("device"))?;
            g.dizin.kaldir(&k.izleyici).await?;
            Ok(json!({}))
        }
        "cikis" | "erisimKapat" => {
            BEKLEYEN.lock().unwrap().take();
            let g = HESAP.lock().unwrap().take();
            afudesk::host_kes();
            afudesk::izleyici_kapat();
            if let Some(g) = g {
                // Revoke locally even if Drive is unreachable.
                google_baglanti::kapat(&g.cihaz.host);
                g.dizin.kaldir(&g.cihaz.izleyici).await?;
            }
            Ok(json!({}))
        }
        _ => Err(anyhow!("operation")),
    }
}

pub(super) fn etkin_hesap() -> Result<Arc<Erisim>> {
    hesap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use afudesk_core::google::Dogrulayici;

    #[test]
    fn panik_tanisi_token_ve_sir_yazmaz() {
        assert_eq!(
            guvenli_panik_mesaji("Expect rustls-platform-verifier to be initialized; secret-token"),
            "rustls-platform-verifier: Android JNI context is not initialized"
        );
        assert_eq!(
            guvenli_panik_mesaji("CryptoProvider: secret-token"),
            "rustls: CryptoProvider is not configured"
        );
        assert_eq!(
            guvenli_panik_mesaji("secret-token"),
            "Rust panic (payload omitted)"
        );
    }

    #[test]
    fn g4_yalniz_sabit_kural_adini_aktarir() {
        let fixture: Value =
            serde_json::from_str(include_str!("../../../../rust/tests/fixtures/google.json"))
                .unwrap();
        let d = Dogrulayici::yeni(
            vec!["test.apps.googleusercontent.com".into()],
            &fixture["jwks"].to_string(),
        )
        .unwrap();
        for (name, reason) in [("wrong_aud", "aud"), ("bad_signature", "imza")] {
            let hata = d
                .dogrula(fixture[name].as_str().unwrap(), 1_700_000_100)
                .unwrap_err();
            assert_eq!(
                guvenli_islem_hatasi(anyhow!(hata).context("G4")).to_string(),
                format!("Google işlemi tamamlanamadı (G4:{reason}), yeniden giriş yap.")
            );
        }
        let hata = anyhow!(afudesk_core::google::DogrulamaHatasi::NONCE)
            .context("token=secret sub=private user@example.com")
            .context("G4");
        assert_eq!(
            guvenli_islem_hatasi(hata).to_string(),
            "Google işlemi tamamlanamadı (G4:nonce), yeniden giriş yap."
        );
        let hata = anyhow!("token=secret").context("G4");
        assert_eq!(
            guvenli_islem_hatasi(hata).to_string(),
            "Google işlemi tamamlanamadı (G4:diger), yeniden giriş yap."
        );
        let hata = anyhow!(afudesk_core::google::DogrulamaHatasi::NONCE).context("G3");
        assert_eq!(
            guvenli_islem_hatasi(hata).to_string(),
            "Google işlemi tamamlanamadı (G3), yeniden giriş yap."
        );
    }

    #[test]
    fn masaustu_sir_zorunludur_ve_hataya_yansitilmaz() {
        for eksik in [None, Some(""), Some(" \r\n\t")] {
            assert_eq!(masaustu_sir(eksik).unwrap_err().to_string(), "G2");
        }
        assert_eq!(
            masaustu_sir(Some("test-desktop-credential")).unwrap(),
            "test-desktop-credential"
        );
    }

    #[cfg(not(target_os = "android"))]
    #[tokio::test]
    async fn masaustu_sir_yoksa_tarayicidan_once_g2_doner() {
        if option_env!("AFUDESK_GOOGLE_DESKTOP_CLIENT_SECRET").is_some_and(|s| !s.trim().is_empty())
        {
            return;
        }
        let hata = google_islem(
            "baslat".into(),
            json!({"desktopClientId":"desktop.apps.googleusercontent.com"}).to_string(),
        )
        .await
        .unwrap_err();
        assert_eq!(
            hata.to_string(),
            "Google girişi bu sürümde yapılandırılmamış (G2)."
        );
    }

    #[test]
    fn android_buildconfig_derleme_ortami_olmadan_audience_saglar() {
        let ayarlar = json!({"webClientId":"test.apps.googleusercontent.com"});
        let ids = istemciler(None, web_istemci(&ayarlar, None));
        let fixture: Value =
            serde_json::from_str(include_str!("../../../../rust/tests/fixtures/google.json"))
                .unwrap();
        let d = Dogrulayici::yeni(ids, &fixture["jwks"].to_string()).unwrap();
        assert!(d
            .dogrula(fixture["token"].as_str().unwrap(), 1_700_000_100)
            .is_ok());
        assert!(d
            .dogrula(fixture["wrong_aud"].as_str().unwrap(), 1_700_000_100)
            .is_err());
        assert_eq!(
            web_istemci(&ayarlar, Some("old.apps.googleusercontent.com")),
            Some("test.apps.googleusercontent.com")
        );
        assert_eq!(
            web_istemci(
                &json!({"webClientId":""}),
                Some("env.apps.googleusercontent.com")
            ),
            Some("env.apps.googleusercontent.com")
        );
    }

    #[tokio::test]
    async fn bilinmeyen_hata_yalniz_guvenli_kod_dondurur() {
        let e = google_islem("secret-operation".into(), "secret-token".into())
            .await
            .unwrap_err()
            .to_string();
        assert!(e.contains("(G9)"));
        assert!(!e.contains("secret"));
    }

    #[test]
    fn web_ve_desktop_audience_kabul_edilir_yabanci_reddedilir() {
        let fixture: Value =
            serde_json::from_str(include_str!("../../../../rust/tests/fixtures/google.json"))
                .unwrap();
        // The signed fixture is used in each role independently: either web or
        // desktop tokens must verify while both IDs are configured on the peer.
        for ids in [
            istemciler(
                Some("desktop.apps.googleusercontent.com"),
                Some("test.apps.googleusercontent.com"),
            ),
            istemciler(
                Some("test.apps.googleusercontent.com"),
                Some("web.apps.googleusercontent.com"),
            ),
        ] {
            assert_eq!(ids.len(), 2);
            let d = Dogrulayici::yeni(ids, &fixture["jwks"].to_string()).unwrap();
            assert!(d
                .dogrula(fixture["token"].as_str().unwrap(), 1_700_000_100)
                .is_ok());
            for name in ["wrong_aud", "bad_signature", "expired", "wrong_iss"] {
                assert!(
                    d.dogrula(fixture[name].as_str().unwrap(), 1_700_000_100)
                        .is_err(),
                    "{name}"
                );
            }
        }
    }

    #[test]
    fn android_istemci_kimligi_audience_olarak_kabul_edilmez() {
        let ayarlar = json!({
            "desktopClientId": "desktop.apps.googleusercontent.com",
            "webClientId": "web.apps.googleusercontent.com",
            "androidClientId": "test.apps.googleusercontent.com",
        });
        let ids = istemciler(
            ayarlar["desktopClientId"].as_str(),
            web_istemci(&ayarlar, None),
        );
        assert_eq!(ids.len(), 2);
        assert!(!ids
            .iter()
            .any(|id| id == ayarlar["androidClientId"].as_str().unwrap()));
        let fixture: Value =
            serde_json::from_str(include_str!("../../../../rust/tests/fixtures/google.json"))
                .unwrap();
        let d = Dogrulayici::yeni(ids, &fixture["jwks"].to_string()).unwrap();
        assert!(d
            .dogrula(fixture["token"].as_str().unwrap(), 1_700_000_100)
            .is_err());
    }

    #[test]
    fn bos_yapilandirma_kimlik_dogrulamasini_acmaz() {
        let fixture: Value =
            serde_json::from_str(include_str!("../../../../rust/tests/fixtures/google.json"))
                .unwrap();
        let ids = istemciler(Some(""), None);
        assert!(ids.is_empty());
        assert!(Dogrulayici::yeni(ids, &fixture["jwks"].to_string()).is_err());
    }
}
