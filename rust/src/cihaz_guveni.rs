//! OAuth tokenlarından bağımsız, yerel iroh anahtarına bağlı cihaz güveni.
//! Çağıran host/viewer anahtarlarını ve hesap/istemci beklentisini güvenilir
//! yerel kaynaktan sağlamalıdır; dış kaydın iddiaları tek başına yetki vermez.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use iroh::{EndpointId, SecretKey, Signature};
use rand::RngCore;
use serde::{Deserialize, Serialize};

const OMUR: i64 = 30 * 24 * 60 * 60;
const LIMIT: usize = 8192;

#[derive(Clone, Copy, Debug, thiserror::Error)]
#[error("G6")]
pub struct GuvenHatasi;

type Sonuc<T> = Result<T, GuvenHatasi>;

// Deliberately no Debug: session identifiers and account bindings stay private.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CihazGuveni {
    v: u8,
    host: String,
    izleyici: String,
    sub: String,
    client_id: String,
    oturum_id: String,
    olusturuldu: i64,
    sona_eriyor: i64,
    son_kullanim: i64,
    imza: String,
}

impl CihazGuveni {
    pub fn yeni(
        host: &SecretKey,
        izleyici: EndpointId,
        sub: &str,
        client_id: &str,
        simdi: i64,
    ) -> Sonuc<Self> {
        let mut rastgele = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut rastgele);
        let mut k = Self {
            v: 1,
            host: host.public().to_string(),
            izleyici: izleyici.to_string(),
            sub: sub.into(),
            client_id: client_id.into(),
            oturum_id: URL_SAFE_NO_PAD.encode(rastgele),
            olusturuldu: simdi,
            sona_eriyor: simdi.checked_add(OMUR).ok_or(GuvenHatasi)?,
            son_kullanim: simdi,
            imza: String::new(),
        };
        k.alanlar()?;
        k.imzala(host)?;
        Ok(k)
    }

    fn alanlar(&self) -> Sonuc<()> {
        if self.v != 1
            || self.host.len() > 64
            || self.izleyici.len() > 64
            || self.host.parse::<EndpointId>().is_err()
            || self.izleyici.parse::<EndpointId>().is_err()
            || self.sub.is_empty()
            || self.sub.len() > 256
            || self.sub.chars().any(char::is_control)
            || self.client_id.len() > 512
            || !self.client_id.ends_with(".apps.googleusercontent.com")
            || self
                .client_id
                .chars()
                .any(|c| c.is_whitespace() || c.is_control())
            || self.oturum_id.len() != 43
            || !URL_SAFE_NO_PAD
                .decode(&self.oturum_id)
                .is_ok_and(|v| v.len() == 32)
            || self.olusturuldu < 0
            || self.olusturuldu.checked_add(OMUR) != Some(self.sona_eriyor)
            || self.son_kullanim < self.olusturuldu
            || self.son_kullanim >= self.sona_eriyor
        {
            return Err(GuvenHatasi);
        }
        Ok(())
    }

    fn imza_verisi(&self) -> Sonuc<Vec<u8>> {
        // Fixed-order, unambiguous encoding with domain separation. imza is excluded.
        serde_json::to_vec(&(
            "afudesk-cihaz-guveni-v1",
            self.v,
            &self.host,
            &self.izleyici,
            &self.sub,
            &self.client_id,
            &self.oturum_id,
            self.olusturuldu,
            self.sona_eriyor,
            self.son_kullanim,
        ))
        .map_err(|_| GuvenHatasi)
    }

    fn imzala(&mut self, host: &SecretKey) -> Sonuc<()> {
        if host.public().to_string() != self.host {
            return Err(GuvenHatasi);
        }
        self.imza = URL_SAFE_NO_PAD.encode(host.sign(&self.imza_verisi()?).to_bytes());
        Ok(())
    }

    pub fn dogrula(
        &self,
        host: EndpointId,
        izleyici: EndpointId,
        sub: &str,
        client_id: &str,
        simdi: i64,
    ) -> Sonuc<()> {
        self.alanlar()?;
        if self.host != host.to_string()
            || self.izleyici != izleyici.to_string()
            || self.sub != sub
            || self.client_id != client_id
            || simdi < self.son_kullanim
            || simdi >= self.sona_eriyor
            || self.imza.len() > 128
        {
            return Err(GuvenHatasi);
        }
        let bytes = URL_SAFE_NO_PAD
            .decode(&self.imza)
            .map_err(|_| GuvenHatasi)?;
        let imza = Signature::try_from(bytes.as_slice()).map_err(|_| GuvenHatasi)?;
        host.verify(&self.imza_verisi()?, &imza)
            .map_err(|_| GuvenHatasi)
    }

    /// Re-sign the high-water clock without moving the original expiry.
    pub fn ilerlet(&mut self, host: &SecretKey, simdi: i64) -> Sonuc<()> {
        let izleyici = self.izleyici.parse().map_err(|_| GuvenHatasi)?;
        self.dogrula(host.public(), izleyici, &self.sub, &self.client_id, simdi)?;
        self.son_kullanim = simdi;
        self.imzala(host)
    }

    /// A stale encrypted bundle can accompany a newer local clock checkpoint.
    /// It must still refer to the exact same immutable, signed transaction.
    pub fn ayni_oturum(&self, diger: &Self) -> bool {
        self.v == diger.v
            && self.host == diger.host
            && self.izleyici == diger.izleyici
            && self.sub == diger.sub
            && self.client_id == diger.client_id
            && self.oturum_id == diger.oturum_id
            && self.olusturuldu == diger.olusturuldu
            && self.sona_eriyor == diger.sona_eriyor
    }

    pub fn oku(veri: &str) -> Sonuc<Self> {
        if veri.len() > LIMIT {
            return Err(GuvenHatasi);
        }
        let k: Self = serde_json::from_str(veri).map_err(|_| GuvenHatasi)?;
        k.alanlar()?;
        Ok(k)
    }

    pub fn yaz(&self) -> Sonuc<String> {
        self.alanlar()?;
        serde_json::to_string(self).map_err(|_| GuvenHatasi)
    }
}
