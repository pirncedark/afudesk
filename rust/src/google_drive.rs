//! Private account directory. Only public device data is uploaded; OAuth tokens
//! stay on the device. Every authorization reads Drive afresh (no stale fallback).
use crate::google::{anahtar_nonce, DogrulamaHatasi, Dogrulanmis, Karar};
use reqwest::{Client, Response};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{fmt, time::Duration};

pub const SCOPE: &str = "https://www.googleapis.com/auth/drive.appdata";
const FILES: &str = "https://www.googleapis.com/drive/v3/files";
const UPLOAD: &str = "https://www.googleapis.com/upload/drive/v3/files";
const LIMIT: usize = 128 * 1024;
const MAX_DEVICES: usize = 256;
pub const HEARTBEAT_SECONDS: i64 = 300;
type Result<T> = std::result::Result<T, DogrulamaHatasi>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CihazKaydi {
    pub v: u8,
    pub sub: String,
    pub host: String,
    pub izleyici: String,
    pub ad: String,
    pub platform: String,
    pub adresler: Vec<String>,
    pub relaylar: Vec<String>,
    pub son_gorulme: i64,
    pub erisim: bool,
}

impl CihazKaydi {
    pub fn guncel(&self, sub: &str, simdi: i64) -> bool {
        self.v == 1
            && self.sub == sub
            && self.son_gorulme <= simdi
            && simdi.saturating_sub(self.son_gorulme) <= HEARTBEAT_SECONDS
    }

    fn dogrula(&self) -> Result<()> {
        if self.v != 1
            || self.sub.is_empty()
            || self.sub.len() > 255
            || self.ad.trim().is_empty()
            || self.ad.len() > 160
            || self.ad.chars().any(char::is_control)
            || self.platform.len() > 32
            || self.host.parse::<iroh::EndpointId>().is_err()
            || self.izleyici.parse::<iroh::EndpointId>().is_err()
            || self.adresler.len() > 32
            || self.relaylar.len() > 8
            || self
                .adresler
                .iter()
                .chain(&self.relaylar)
                .any(|s| s.len() > 512)
            || self.son_gorulme < 0
        {
            return Err(DogrulamaHatasi);
        }
        Ok(())
    }

    fn dosya_adi(&self) -> String {
        format!("afudesk-device-{}.json", self.izleyici)
    }
}

/// Peer identity comes from QUIC, never a message field. Same-account devices
/// need a current directory record; other accounts retain the old approval path.
pub fn drive_karari(
    yerel: &Dogrulanmis,
    karsi: std::result::Result<&Dogrulanmis, ()>,
    uzak_anahtar: &str,
    kayitlar: &[CihazKaydi],
    simdi: i64,
) -> Karar {
    let Ok(karsi) = karsi else {
        return Karar::Red;
    };
    if simdi < 0 || yerel.gecerlilik_sonu() <= simdi || karsi.gecerlilik_sonu() <= simdi {
        return Karar::Red;
    }
    if yerel.sub() != karsi.sub() {
        return Karar::OnaySor;
    }
    let mut eslesen = kayitlar
        .iter()
        .filter(|k| k.izleyici == uzak_anahtar && k.guncel(yerel.sub(), simdi));
    let Some(k) = eslesen.next() else {
        return Karar::Red;
    };
    if eslesen.next().is_some() {
        return Karar::Red;
    }
    let bag = anahtar_nonce(&k.host, &k.izleyici);
    let nonce = karsi.nonce();
    if nonce == bag
        || nonce.strip_prefix(&format!("{bag}.")).is_some_and(|s| {
            use base64::Engine;
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(s)
                .is_ok_and(|b| b.len() == 32)
        })
    {
        Karar::Kabul
    } else {
        Karar::Red
    }
}

/// Access token is deliberately neither serializable nor printable.
pub struct DriveDizin {
    client: Client,
    token: String,
    sub: String,
    files: String,
    upload: String,
}
impl fmt::Debug for DriveDizin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DriveDizin([gizli])")
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Sayfa {
    #[serde(default)]
    files: Vec<Dosya>,
    next_page_token: Option<String>,
}
#[derive(Deserialize)]
struct Dosya {
    id: String,
    name: String,
}

async fn json<T: DeserializeOwned>(response: Response) -> Result<T> {
    let mut r = response.error_for_status().map_err(|_| DogrulamaHatasi)?;
    let mut body = Vec::new();
    while let Some(c) = r.chunk().await.map_err(|_| DogrulamaHatasi)? {
        if body.len().saturating_add(c.len()) > LIMIT {
            return Err(DogrulamaHatasi);
        }
        body.extend_from_slice(&c);
    }
    serde_json::from_slice(&body).map_err(|_| DogrulamaHatasi)
}

impl DriveDizin {
    /// `hesap` must be the verified identity from the OAuth transaction returning
    /// this access token. An ID token is not a Drive access token.
    pub fn yeni(access_token: String, hesap: &Dogrulanmis) -> Result<Self> {
        if access_token.is_empty() || access_token.len() > 16 * 1024 {
            return Err(DogrulamaHatasi);
        }
        Ok(Self {
            client: Client::builder()
                .https_only(true)
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(10))
                .build()
                .map_err(|_| DogrulamaHatasi)?,
            token: access_token,
            sub: hesap.sub().into(),
            files: FILES.into(),
            upload: UPLOAD.into(),
        })
    }

    async fn dosyalar(&self) -> Result<Vec<Dosya>> {
        let mut out = Vec::new();
        let mut page = String::new();
        let mut seen = std::collections::HashSet::new();
        loop {
            let mut req = self
                .client
                .get(&self.files)
                .bearer_auth(&self.token)
                .query(&[
                    ("spaces", "appDataFolder"),
                    ("q", "trashed = false and name contains 'afudesk-device-'"),
                    ("fields", "nextPageToken,files(id,name)"),
                    ("pageSize", "100"),
                ]);
            if !page.is_empty() {
                req = req.query(&[("pageToken", page.as_str())]);
            }
            let p: Sayfa = json(req.send().await.map_err(|_| DogrulamaHatasi)?).await?;
            for f in &p.files {
                if f.id.is_empty()
                    || f.id.len() > 256
                    || !f
                        .id
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
                {
                    return Err(DogrulamaHatasi);
                }
            }
            out.extend(p.files);
            if out.len() > MAX_DEVICES {
                return Err(DogrulamaHatasi);
            }
            match p.next_page_token {
                None => break,
                Some(p) if p.is_empty() || p.len() > 4096 || !seen.insert(p.clone()) => {
                    return Err(DogrulamaHatasi)
                }
                Some(p) => page = p,
            }
            if seen.len() > MAX_DEVICES {
                return Err(DogrulamaHatasi);
            }
        }
        Ok(out)
    }

    pub async fn liste(&self) -> Result<Vec<CihazKaydi>> {
        let files = self.dosyalar().await?;
        let mut out = Vec::new();
        for f in files {
            let k: CihazKaydi = json(
                self.client
                    .get(format!("{}/{}", self.files, f.id))
                    .bearer_auth(&self.token)
                    .query(&[("alt", "media")])
                    .send()
                    .await
                    .map_err(|_| DogrulamaHatasi)?,
            )
            .await?;
            k.dogrula()?;
            if k.sub != self.sub || k.dosya_adi() != f.name {
                return Err(DogrulamaHatasi);
            }
            out.push(k);
        }
        Ok(out)
    }

    /// One record per device; updates do not overwrite other devices' records.
    /// Call only during an explicit sign-in. Heartbeats must use `yenile` so a
    /// removed device cannot silently recreate its membership.
    pub async fn kaydet(&self, kayit: &CihazKaydi) -> Result<()> {
        self.yaz(kayit, true).await
    }

    pub async fn yenile(&self, kayit: &CihazKaydi) -> Result<()> {
        self.yaz(kayit, false).await
    }

    async fn yaz(&self, kayit: &CihazKaydi, yeni_kayit: bool) -> Result<()> {
        kayit.dogrula()?;
        if kayit.sub != self.sub {
            return Err(DogrulamaHatasi);
        }
        let files = self.dosyalar().await?;
        let found: Vec<_> = files
            .iter()
            .filter(|f| f.name == kayit.dosya_adi())
            .collect();
        if found.len() > 1 {
            return Err(DogrulamaHatasi);
        }
        let response = if let Some(f) = found.first() {
            self.client
                .patch(format!("{}/{}", self.upload, f.id))
                .bearer_auth(&self.token)
                .query(&[("uploadType", "media")])
                .json(kayit)
                .send()
                .await
        } else {
            if !yeni_kayit {
                return Err(DogrulamaHatasi);
            }
            // Random multipart boundary cannot occur inside this bounded JSON.
            let boundary = format!("afudesk-{}", crate::kod::yeni_bilet());
            let metadata = serde_json::json!({"name":kayit.dosya_adi(),"parents":["appDataFolder"],"mimeType":"application/json"});
            let data = serde_json::to_string(kayit).map_err(|_| DogrulamaHatasi)?;
            let body = format!("--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{metadata}\r\n--{boundary}\r\nContent-Type: application/json\r\n\r\n{data}\r\n--{boundary}--\r\n");
            self.client
                .post(&self.upload)
                .bearer_auth(&self.token)
                .query(&[("uploadType", "multipart")])
                .header(
                    "Content-Type",
                    format!("multipart/related; boundary={boundary}"),
                )
                .body(body)
                .send()
                .await
        };
        response
            .and_then(|r| r.error_for_status())
            .map_err(|_| DogrulamaHatasi)?;
        Ok(())
    }

    pub async fn kaldir(&self, izleyici: &str) -> Result<()> {
        izleyici
            .parse::<iroh::EndpointId>()
            .map_err(|_| DogrulamaHatasi)?;
        let name = format!("afudesk-device-{izleyici}.json");
        for f in self.dosyalar().await?.iter().filter(|f| f.name == name) {
            self.client
                .delete(format!("{}/{}", self.files, f.id))
                .bearer_auth(&self.token)
                .send()
                .await
                .and_then(|r| r.error_for_status())
                .map_err(|_| DogrulamaHatasi)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn heartbeat_silinmis_cihazi_yeniden_kaydetmez() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                request.push(stream.read_u8().await.unwrap());
                assert!(request.len() < 8192);
            }
            let request = String::from_utf8(request).unwrap();
            assert!(request.starts_with("GET /files?"));
            assert!(request
                .to_lowercase()
                .contains("authorization: bearer test-only"));
            let body = r#"{"files":[]}"#;
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            // Any attempt to POST a replacement would fail this test.
            assert!(
                tokio::time::timeout(Duration::from_millis(150), listener.accept())
                    .await
                    .is_err()
            );
        });
        let dizin = DriveDizin {
            client: Client::new(),
            token: "test-only".into(),
            sub: "account".into(),
            files: format!("{url}/files"),
            upload: format!("{url}/upload"),
        };
        let key = iroh::SecretKey::from_bytes(&[7; 32]).public().to_string();
        let k = CihazKaydi {
            v: 1,
            sub: "account".into(),
            host: key.clone(),
            izleyici: key,
            ad: "Ev".into(),
            platform: "windows".into(),
            adresler: vec![],
            relaylar: vec![],
            son_gorulme: 1,
            erisim: true,
        };
        assert!(dizin.yenile(&k).await.is_err());
        server.await.unwrap();
    }
}
