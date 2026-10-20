//! Live Google authorization shared by host and viewer. No PIN or approval for
//! same-account members. Installing/removing an identity never changes old grants.
use crate::{
    google::{anahtar_nonce, DogrulamaHatasi, Dogrulanmis, Dogrulayici, Karar},
    google_drive::{drive_karari, CihazKaydi, DriveDizin},
};
use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, OnceLock, RwLock},
};

#[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Kanit(pub String);
impl fmt::Debug for Kanit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("GoogleKaniti([gizli])")
    }
}

pub struct Erisim {
    pub hesap: Dogrulanmis,
    pub dizin: DriveDizin,
    pub cihaz: CihazKaydi,
    pub istemciler: Vec<String>,
    kanit: Kanit,
}
impl Erisim {
    /// The OAuth transaction must also check its exact random nonce before this
    /// call. Here the signed nonce additionally binds the two persistent keys.
    pub async fn yeni(
        id_token: String,
        access_token: String,
        istemciler: Vec<String>,
        mut cihaz: CihazKaydi,
    ) -> Result<Arc<Self>, DogrulamaHatasi> {
        let d = Dogrulayici::google_dan(istemciler.clone()).await?;
        let hesap = d.dogrula(&id_token, crate::kod::simdi())?;
        let bag = anahtar_nonce(&cihaz.host, &cihaz.izleyici);
        if !hesap.nonce().starts_with(&format!("{bag}.")) {
            return Err(DogrulamaHatasi);
        }
        cihaz.sub = hesap.sub().into();
        let dizin = DriveDizin::yeni(access_token, &hesap)?;
        Ok(Arc::new(Self {
            hesap,
            dizin,
            cihaz,
            istemciler,
            kanit: Kanit(id_token),
        }))
    }

    pub async fn yayinla(&self) -> Result<(), DogrulamaHatasi> {
        let simdi = crate::kod::simdi();
        if self.hesap.gecerlilik_sonu() <= simdi {
            return Err(DogrulamaHatasi);
        }
        let mut kayit = self.cihaz.clone();
        kayit.son_gorulme = simdi;
        self.dizin.yenile(&kayit).await
    }

    /// Fresh JWKS and fresh Drive membership. The peer-supplied name and sub do
    /// not participate in granting access. A directory outage rejects the call.
    pub async fn yetki(
        &self,
        kanit: &Kanit,
        uzak: &str,
    ) -> Result<(Karar, String), DogrulamaHatasi> {
        let d = Dogrulayici::google_dan(self.istemciler.clone()).await?;
        let karsi = d.dogrula(&kanit.0, crate::kod::simdi())?;
        if self.hesap.sub() != karsi.sub() {
            return Ok((Karar::OnaySor, String::new()));
        }
        let liste = self.dizin.liste().await?;
        // A removed/disabled local device cannot keep hosting from memory.
        if !liste.iter().any(|k| {
            k.host == self.cihaz.host
                && k.izleyici == self.cihaz.izleyici
                && k.erisim
                && k.guncel(self.hesap.sub(), crate::kod::simdi())
        }) {
            return Err(DogrulamaHatasi);
        }
        let karar = drive_karari(&self.hesap, Ok(&karsi), uzak, &liste, crate::kod::simdi());
        let ad = liste
            .iter()
            .find(|k| k.izleyici == uzak)
            .map(|k| k.ad.clone())
            .unwrap_or_default();
        Ok((karar, ad))
    }

    pub fn kanit(&self) -> Kanit {
        self.kanit.clone()
    }
}

fn kayitlar() -> &'static RwLock<HashMap<String, Arc<Erisim>>> {
    static K: OnceLock<RwLock<HashMap<String, Arc<Erisim>>>> = OnceLock::new();
    K.get_or_init(Default::default)
}
pub fn etkinlestir(erisim: Arc<Erisim>) {
    kayitlar()
        .write()
        .expect("google lock")
        .insert(erisim.cihaz.host.clone(), erisim);
}
pub fn kapat(host: &str) {
    kayitlar().write().expect("google lock").remove(host);
}
pub fn bul(host: &str) -> Option<Arc<Erisim>> {
    kayitlar().read().expect("google lock").get(host).cloned()
}
