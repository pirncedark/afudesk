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
static BEKLEYEN: Mutex<Option<Bekleyen>> = Mutex::new(None);
static HESAP: Mutex<Option<Arc<Erisim>>> = Mutex::new(None);
// Serializes sign-in, sign-out and directory mutations across await points.
static ISLEM: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn istemciler() -> Vec<String> {
    [
        option_env!("AFUDESK_GOOGLE_DESKTOP_CLIENT_ID"),
        option_env!("AFUDESK_GOOGLE_WEB_CLIENT_ID"),
    ]
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
            let k = cihaz()?;
            if cfg!(target_os = "android") {
                let id = option_env!("AFUDESK_GOOGLE_WEB_CLIENT_ID")
                    .ok_or_else(|| anyhow!("configuration"))?;
                let g = Giris::yeni(id, 1, &k.host, &k.izleyici)?;
                let nonce = g.nonce().to_owned();
                *BEKLEYEN.lock().unwrap() = Some(Bekleyen::Android(g));
                Ok(json!({"nonce":nonce}))
            } else {
                let id = option_env!("AFUDESK_GOOGLE_DESKTOP_CLIENT_ID")
                    .ok_or_else(|| anyhow!("configuration"))?;
                let g = MasaustuGiris::baslat(id, &k.host, &k.izleyici).await?;
                let url = g.url().to_owned();
                *BEKLEYEN.lock().unwrap() = Some(Bekleyen::Masaustu(g));
                Ok(json!({"url":url}))
            }
        }
        "iptal" => {
            BEKLEYEN.lock().unwrap().take();
            Ok(json!({}))
        }
        "tamamla" => {
            let pending = BEKLEYEN
                .lock()
                .unwrap()
                .take()
                .ok_or_else(|| anyhow!("no transaction"))?;
            let (id, access) = match pending {
                Bekleyen::Masaustu(g) => {
                    let token = g
                        .kod_bekle()
                        .await?
                        .oturum(option_env!("AFUDESK_GOOGLE_DESKTOP_CLIENT_SECRET"))
                        .await?;
                    (token.id_token, token.access_token)
                }
                Bekleyen::Android(g) => {
                    let v: Value = serde_json::from_str(veri)?;
                    let id = v["idToken"].as_str().ok_or_else(|| anyhow!("id"))?;
                    let d = afudesk_core::google::Dogrulayici::google_dan(istemciler()).await?;
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
            let g = Erisim::yeni(id, access, istemciler(), k).await?;
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
