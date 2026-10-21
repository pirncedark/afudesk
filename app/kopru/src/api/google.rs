//! Account operations contain no printable credentials. JSON is only a small
//! bridge DTO; authorization always stays inside the core.
use super::afudesk;
use afudesk_core::{
    google_baglanti::{self, Erisim},
    google_drive::CihazKaydi,
    google_oauth::{Giris, MasaustuGiris},
    kimlik,
};
use anyhow::{anyhow, ensure, Result};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

enum Bekleyen {
    Masaustu(MasaustuGiris),
    Android(Giris),
}
// Keep the audience allowlist with the transaction until remote access starts.
static BEKLEYEN: Mutex<Option<(Bekleyen, Vec<String>)>> = Mutex::new(None);
static HESAP: Mutex<Option<Arc<Erisim>>> = Mutex::new(None);
// Serializes sign-in, sign-out and directory mutations across await points.
static ISLEM: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

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

/// No tokens are returned except the native sign-in nonce. All failures are
/// deliberately reduced to one user-facing sentence at this boundary.
pub async fn google_islem(islem: String, veri: String) -> Result<String> {
    islem_yap(&islem, &veri)
        .await
        .map(|v| v.to_string())
        .map_err(|_| anyhow!("Google işlemi tamamlanamadı, yeniden giriş yap."))
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
            let web = option_env!("AFUDESK_GOOGLE_WEB_CLIENT_ID");
            let kabul_edilen = istemciler(desktop, web);
            let k = cihaz()?;
            if cfg!(target_os = "android") {
                let id = web.ok_or_else(|| anyhow!("configuration"))?;
                let g = Giris::yeni(id, 1, &k.host, &k.izleyici)?;
                let nonce = g.nonce().to_owned();
                *BEKLEYEN.lock().unwrap() = Some((Bekleyen::Android(g), kabul_edilen));
                Ok(json!({"nonce":nonce}))
            } else {
                let id = desktop.ok_or_else(|| anyhow!("configuration"))?;
                let g = MasaustuGiris::baslat(id, &k.host, &k.izleyici).await?;
                let url = g.url().to_owned();
                *BEKLEYEN.lock().unwrap() = Some((Bekleyen::Masaustu(g), kabul_edilen));
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
                Bekleyen::Masaustu(g) => {
                    let token = g.kod_bekle().await?.oturum(None).await?;
                    (token.id_token, token.access_token)
                }
                Bekleyen::Android(g) => {
                    let v: Value = serde_json::from_str(veri)?;
                    let id = v["idToken"].as_str().ok_or_else(|| anyhow!("id"))?;
                    let d =
                        afudesk_core::google::Dogrulayici::google_dan(kabul_edilen.clone()).await?;
                    let identity = d.dogrula(id, afudesk_core::kod::simdi())?;
                    ensure!(identity.nonce() == g.nonce(), "nonce");
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
            let g = Erisim::yeni(id, access, kabul_edilen, k).await?;
            g.dizin.hesabi_dogrula().await?;
            g.dizin.kaydet(&g.cihaz).await?;
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
    fn bos_yapilandirma_kimlik_dogrulamasini_acmaz() {
        let fixture: Value =
            serde_json::from_str(include_str!("../../../../rust/tests/fixtures/google.json"))
                .unwrap();
        let ids = istemciler(Some(""), None);
        assert!(ids.is_empty());
        assert!(Dogrulayici::yeni(ids, &fixture["jwks"].to_string()).is_err());
    }
}
