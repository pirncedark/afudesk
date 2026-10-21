//! Masaüstü OAuth işlemi: yalnız sistem tarayıcısı + loopback, PKCE S256.
//! İşlem sahipliği tüketilir; aynı callback ikinci kez kullanılamaz. Bu modül
//! hesap dizinine kayıt yapmaz ve tek başına uzaktan erişim izni vermez.
use crate::google::{anahtar_nonce, DogrulamaHatasi, Dogrulanmis, Dogrulayici};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use reqwest::Url;
use sha2::{Digest, Sha256};
use std::{
    fmt,
    time::{Duration, Instant},
};

type Sonuc<T> = Result<T, DogrulamaHatasi>;
const OMUR: Duration = Duration::from_secs(300);

pub struct Giris {
    client_id: String,
    redirect: String,
    verifier: String,
    state: String,
    nonce: String,
    url: String,
    baslangic: Instant,
}

fn rastgele() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

impl Giris {
    /// Port çağıranın önceden açtığı 127.0.0.1 dinleyicisine ait olmalıdır.
    pub fn yeni(client_id: &str, port: u16, host: &str, viewer: &str) -> Sonuc<Self> {
        let nonce = format!("{}.{}", anahtar_nonce(host, viewer), rastgele());
        Self::dizin_nonce(client_id, port, host, viewer, &nonce)
    }

    /// Dizin tarafından açılan tek kullanımlık işlemin nonce'u Google'a aynen
    /// aktarılır. Çağıran dizin bağlantısının kimliğini ayrıca doğrulamalıdır.
    pub fn dizin_nonce(
        client_id: &str,
        port: u16,
        host: &str,
        viewer: &str,
        nonce: &str,
    ) -> Sonuc<Self> {
        let beklenen = anahtar_nonce(host, viewer);
        let (bag, nonce_rastgele) = nonce.split_once('.').ok_or(DogrulamaHatasi)?;
        if bag != beklenen
            || URL_SAFE_NO_PAD
                .decode(nonce_rastgele)
                .map_err(|_| DogrulamaHatasi)?
                .len()
                != 32
        {
            return Err(DogrulamaHatasi);
        }
        if port == 0
            || host.is_empty()
            || viewer.is_empty()
            || !client_id.ends_with(".apps.googleusercontent.com")
            || client_id.chars().any(char::is_whitespace)
        {
            return Err(DogrulamaHatasi);
        }
        let redirect = format!("http://127.0.0.1:{port}/oauth2/callback");
        let verifier = rastgele();
        let state = rastgele();
        // Rastgele işlem nonce'u + kalıcı anahtar bağlamı; sabit nonce replay'i yok.
        let nonce = nonce.to_owned();
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let mut url = Url::parse("https://accounts.google.com/o/oauth2/v2/auth")
            .map_err(|_| DogrulamaHatasi)?;
        url.query_pairs_mut().extend_pairs([
            ("client_id", client_id),
            ("redirect_uri", redirect.as_str()),
            ("response_type", "code"),
            (
                "scope",
                "openid email profile https://www.googleapis.com/auth/drive.appdata",
            ),
            ("state", state.as_str()),
            ("nonce", nonce.as_str()),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ]);
        Ok(Self {
            client_id: client_id.into(),
            redirect,
            verifier,
            state,
            nonce,
            url: url.into(),
            baslangic: Instant::now(),
        })
    }

    pub fn url(&self) -> &str {
        &self.url
    }
    pub fn nonce(&self) -> &str {
        &self.nonce
    }

    /// HTTP sunucu yalnız GET istek hedefini buraya verir; tam URL kabul edilmez.
    pub fn callback(self, hedef: &str) -> Sonuc<Kod> {
        if self.baslangic.elapsed() > OMUR
            || hedef.len() > 8192
            || !hedef.starts_with("/oauth2/callback?")
            || hedef.contains('#')
        {
            return Err(DogrulamaHatasi);
        }
        let url = Url::parse(&format!("http://127.0.0.1{hedef}")).map_err(|_| DogrulamaHatasi)?;
        let mut state = None;
        let mut code = None;
        for (k, v) in url.query_pairs() {
            match k.as_ref() {
                "state" if state.is_none() => state = Some(v.into_owned()),
                "code" if code.is_none() => code = Some(v.into_owned()),
                "state" | "code" | "error" => return Err(DogrulamaHatasi),
                _ => (),
            }
        }
        if state.as_deref() != Some(self.state.as_str()) {
            return Err(DogrulamaHatasi);
        }
        let code = code
            .filter(|c| !c.is_empty() && c.len() <= 4096)
            .ok_or(DogrulamaHatasi)?;
        Ok(Kod { giris: self, code })
    }
}

pub struct Kod {
    giris: Giris,
    code: String,
}

/// URL oluşturulmadan loopback dinleyicisi açılır. Servis/autostart kurulmaz.
pub struct MasaustuGiris {
    giris: Giris,
    listener: tokio::net::TcpListener,
}

impl MasaustuGiris {
    pub async fn baslat(client_id: &str, host: &str, viewer: &str) -> Sonuc<Self> {
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .map_err(|_| DogrulamaHatasi)?;
        let port = listener.local_addr().map_err(|_| DogrulamaHatasi)?.port();
        Ok(Self {
            giris: Giris::yeni(client_id, port, host, viewer)?,
            listener,
        })
    }

    /// Çağıran bunu sistem tarayıcısında açar; gömülü webview kullanılmaz.
    pub fn url(&self) -> &str {
        self.giris.url()
    }

    pub async fn kod_bekle(self) -> Sonuc<Kod> {
        tokio::time::timeout(
            OMUR.saturating_sub(self.giris.baslangic.elapsed()),
            self.al(),
        )
        .await
        .map_err(|_| DogrulamaHatasi)?
    }

    async fn al(self) -> Sonuc<Kod> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let port = self
            .listener
            .local_addr()
            .map_err(|_| DogrulamaHatasi)?
            .port();
        let (mut stream, remote) = self.listener.accept().await.map_err(|_| DogrulamaHatasi)?;
        if !remote.ip().is_loopback() {
            return Err(DogrulamaHatasi);
        }
        let request = tokio::time::timeout(Duration::from_secs(5), async {
            let mut bytes = Vec::new();
            loop {
                if bytes.len() >= 8192 {
                    return Err(DogrulamaHatasi);
                }
                bytes.push(stream.read_u8().await.map_err(|_| DogrulamaHatasi)?);
                if bytes.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            String::from_utf8(bytes).map_err(|_| DogrulamaHatasi)
        })
        .await
        .map_err(|_| DogrulamaHatasi)??;
        let mut lines = request.split("\r\n");
        let first: Vec<_> = lines.next().unwrap_or_default().split(' ').collect();
        let mut hosts = lines
            .filter_map(|l| l.split_once(':'))
            .filter(|(k, _)| k.eq_ignore_ascii_case("host"));
        let expected_host = format!("127.0.0.1:{port}");
        if first.len() != 3
            || first[0] != "GET"
            || first[2] != "HTTP/1.1"
            || hosts.next().map(|(_, v)| v.trim()) != Some(expected_host.as_str())
            || hosts.next().is_some()
        {
            return Err(DogrulamaHatasi);
        }
        let result = self.giris.callback(first[1]);
        let (status, body) = if result.is_ok() {
            ("200 OK", "AfuDesk'e donebilirsin.")
        } else {
            (
                "400 Bad Request",
                "Giris tamamlanamadi. AfuDesk'te yeniden dene.",
            )
        };
        // URL/token/state yansıtılmaz; harici kaynak ve tarayıcı cache'i yok.
        let response = format!("HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'\r\nReferrer-Policy: no-referrer\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}", body.len());
        let _ = tokio::time::timeout(
            Duration::from_secs(2),
            stream.write_all(response.as_bytes()),
        )
        .await;
        result
    }
}

impl Kod {
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Google token endpoint ve JWKS dışında hedefe gidilmez; redirect yoktur.
    /// Token veya HTTP hata gövdesi hataya/loga eklenmez. İstemci metadata'sı
    /// dağıtımda sağlanır, repo içinde gerçek client_secret bulunmaz.
    pub async fn dogrula(self, client_secret: Option<&str>) -> Sonuc<Dogrulanmis> {
        Ok(self.oturum(client_secret).await?.hesap)
    }

    /// Returns Drive credentials only after signature, audience and the exact
    /// one-use OAuth nonce have all been checked. Never log this result.
    pub async fn oturum(self, client_secret: Option<&str>) -> Sonuc<GoogleOturumu> {
        if self.giris.baslangic.elapsed() > OMUR {
            return Err(DogrulamaHatasi);
        }
        let client = crate::google::http_client()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|_| DogrulamaHatasi)?;
        let mut form = vec![
            ("client_id", self.giris.client_id.as_str()),
            ("redirect_uri", self.giris.redirect.as_str()),
            ("code_verifier", self.giris.verifier.as_str()),
            ("grant_type", "authorization_code"),
            ("code", self.code.as_str()),
        ];
        if let Some(s) = client_secret {
            form.push(("client_secret", s));
        }
        // reqwest form özelliğine ek bağımlılık yerine standart URL kodlaması.
        let mut body = Url::parse("https://localhost/").map_err(|_| DogrulamaHatasi)?;
        body.query_pairs_mut().extend_pairs(form);
        let mut response = client
            .post("https://oauth2.googleapis.com/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body.query().unwrap_or_default().to_owned())
            .send()
            .await
            .map_err(|_| DogrulamaHatasi)?
            .error_for_status()
            .map_err(|_| DogrulamaHatasi)?;
        let mut bytes = Vec::new();
        while let Some(c) = response.chunk().await.map_err(|_| DogrulamaHatasi)? {
            if bytes.len().saturating_add(c.len()) > 64 * 1024 {
                return Err(DogrulamaHatasi);
            }
            bytes.extend_from_slice(&c);
        }
        #[derive(serde::Deserialize)]
        struct Token {
            id_token: String,
            access_token: String,
            token_type: String,
        }
        let token: Token = serde_json::from_slice(&bytes).map_err(|_| DogrulamaHatasi)?;
        if token.access_token.is_empty()
            || token.access_token.len() > 16 * 1024
            || !token.token_type.eq_ignore_ascii_case("Bearer")
        {
            return Err(DogrulamaHatasi);
        }
        let verifier = Dogrulayici::google_dan(vec![self.giris.client_id]).await?;
        let simdi = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| DogrulamaHatasi)?
            .as_secs()
            .try_into()
            .map_err(|_| DogrulamaHatasi)?;
        let hesap = verifier.dogrula(&token.id_token, simdi)?;
        if hesap.nonce() != self.giris.nonce {
            return Err(DogrulamaHatasi);
        }
        Ok(GoogleOturumu {
            hesap,
            id_token: token.id_token,
            access_token: token.access_token,
        })
    }
}

pub struct GoogleOturumu {
    pub hesap: Dogrulanmis,
    pub id_token: String,
    pub access_token: String,
}
impl fmt::Debug for GoogleOturumu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("GoogleOturumu([gizli])")
    }
}

impl fmt::Debug for Giris {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Giris([gizli])")
    }
}
impl fmt::Debug for Kod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Kod([gizli])")
    }
}
