//! Dağıtım bağımsız cihaz dizini çekirdeği; ağ sunucusu değildir.
//! Yalnız Google imzası doğrulanmış hesap + her iki iroh anahtarının sahipliği
//! kayıt açabilir. Üretim adaptörü bunu seri işlem/kalıcı depo altında çalıştırır.
//! `liste` ve `kaldir` yalnız kimliği doğrulanmış yönetim bağlantısına açılmalıdır.
use crate::google::{anahtar_nonce, DogrulamaHatasi, Dogrulanmis};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use iroh::EndpointId;
use rand::RngCore;
use ring::signature::{UnparsedPublicKey, ED25519};
use std::collections::HashMap;

type Sonuc<T> = Result<T, DogrulamaHatasi>;
const BEKLEYEN_AZAMI: usize = 1024;
const CIHAZ_AZAMI: usize = 10_000;

struct Bekleyen {
    host: EndpointId,
    izleyici: EndpointId,
    baslangic: i64,
}

/// Hesap kimliği/token taşımayan, yalnız görünen cihaz bilgisi.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cihaz {
    pub host: EndpointId,
    pub izleyici: EndpointId,
    pub ad: String,
}

struct Kayit {
    sub: String,
    cihaz: Cihaz,
}

#[derive(Default)]
pub struct Dizin {
    bekleyen: HashMap<String, Bekleyen>,
    kayitlar: HashMap<EndpointId, Kayit>,
}

impl Dizin {
    /// Dönen nonce Google giriş isteğine aynen verilir. 256 bit rastgelelik;
    /// süreli ve tek kullanımlık. Host/izleyici anahtarı kimliği nonce'a bağlıdır.
    pub fn baslat(&mut self, host: EndpointId, izleyici: EndpointId, simdi: i64) -> Sonuc<String> {
        if simdi < 0 || host == izleyici {
            return Err(DogrulamaHatasi);
        }
        self.bekleyen
            .retain(|_, b| simdi >= b.baslangic && simdi - b.baslangic < 300);
        if self.bekleyen.len() >= BEKLEYEN_AZAMI {
            return Err(DogrulamaHatasi);
        }
        let mut bytes = [0; 32];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        let nonce = format!(
            "{}.{}",
            anahtar_nonce(&host.to_string(), &izleyici.to_string()),
            URL_SAFE_NO_PAD.encode(bytes)
        );
        self.bekleyen.insert(
            nonce.clone(),
            Bekleyen {
                host,
                izleyici,
                baslangic: simdi,
            },
        );
        Ok(nonce)
    }

    /// İmza alanı sürümlü ve alan ayrımlıdır; ad değiştirilemez. İstemci yalnız
    /// kendisinin başlattığı giriş nonce'unu imzalamalıdır.
    pub fn kayit_mesaji(nonce: &str, ad: &str) -> Vec<u8> {
        let mut out = b"afudesk-google-register-v1\0".to_vec();
        out.extend_from_slice(&(nonce.len() as u64).to_be_bytes());
        out.extend_from_slice(nonce.as_bytes());
        out.extend_from_slice(ad.as_bytes());
        out
    }

    pub fn kaydet(
        &mut self,
        hesap: &Dogrulanmis,
        ad: &str,
        host_imza: &[u8],
        izleyici_imza: &[u8],
        simdi: i64,
    ) -> Sonuc<()> {
        // Hatalı deneme de challenge'ı tüketir. Token/kayıt replay edilemez.
        let b = self.bekleyen.remove(hesap.nonce()).ok_or(DogrulamaHatasi)?;
        if simdi < b.baslangic
            || simdi - b.baslangic >= 300
            || hesap.gecerlilik_sonu() <= simdi
            || ad.trim().is_empty()
            || ad.chars().count() > 64
            || ad.chars().any(char::is_control)
        {
            return Err(DogrulamaHatasi);
        }
        let mesaj = Self::kayit_mesaji(hesap.nonce(), ad);
        for (key, imza) in [(b.host, host_imza), (b.izleyici, izleyici_imza)] {
            UnparsedPublicKey::new(&ED25519, key.as_bytes())
                .verify(&mesaj, imza)
                .map_err(|_| DogrulamaHatasi)?;
        }
        // Bir anahtar başka cihaz/hesapla sessizce yeniden bağlanamaz.
        if self.kayitlar.values().any(|k| {
            let c = &k.cihaz;
            (c.host == b.host
                || c.izleyici == b.host
                || c.host == b.izleyici
                || c.izleyici == b.izleyici)
                && (k.sub != hesap.sub() || c.host != b.host || c.izleyici != b.izleyici)
        }) || (!self.kayitlar.contains_key(&b.host) && self.kayitlar.len() >= CIHAZ_AZAMI)
        {
            return Err(DogrulamaHatasi);
        }
        self.kayitlar.insert(
            b.host,
            Kayit {
                sub: hesap.sub().into(),
                cihaz: Cihaz {
                    host: b.host,
                    izleyici: b.izleyici,
                    ad: ad.into(),
                },
            },
        );
        Ok(())
    }

    pub fn liste(&self, hesap: &Dogrulanmis, simdi: i64) -> Sonuc<Vec<Cihaz>> {
        if simdi < 0 || hesap.gecerlilik_sonu() <= simdi {
            return Err(DogrulamaHatasi);
        }
        let mut cihazlar: Vec<_> = self
            .kayitlar
            .values()
            .filter(|k| k.sub == hesap.sub())
            .map(|k| k.cihaz.clone())
            .collect();
        cihazlar.sort_by(|a, b| a.ad.cmp(&b.ad).then_with(|| a.host.cmp(&b.host)));
        Ok(cihazlar)
    }

    /// Güvenilir yönetim katmanı çağırır; istemcinin gönderdiği sub kullanılmaz.
    /// Kaldırmadan önce çıkarılan cihazın bekleyen kayıt işlemleri de iptal edilir.
    pub fn kaldir(&mut self, sub: &str, host: EndpointId) {
        if self.kayitlar.get(&host).is_some_and(|k| k.sub == sub) {
            if let Some(k) = self.kayitlar.remove(&host) {
                self.bekleyen
                    .retain(|_, b| b.host != host && b.izleyici != k.cihaz.izleyici);
            }
        }
    }

    /// Karar anında güncel dizinden kontrol edilir; istemciden bool alınmaz.
    pub fn kayitli(&self, sub: &str, host: EndpointId, izleyici: EndpointId) -> bool {
        self.kayitlar.get(&host).is_some_and(|k| k.sub == sub)
            && self
                .kayitlar
                .values()
                .any(|k| k.sub == sub && k.cihaz.izleyici == izleyici)
    }
}
