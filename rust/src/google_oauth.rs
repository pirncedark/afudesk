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

// No public endpoint override exists. Only unit tests can inject loopback HTTP
// and fixture JWKS/time; production always constructs a fixed Google HTTPS client.
struct TokenIstemcisi {
    http: reqwest::Client,
    hedef: String,
    #[cfg(test)]
    test_jwks: Option<String>,
    #[cfg(test)]
    test_simdi: Option<i64>,
}

impl TokenIstemcisi {
    fn google() -> Sonuc<Self> {
        Ok(Self {
            http: crate::google::http_client()?
                .https_only(true)
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(20))
                .build()
                .map_err(|_| DogrulamaHatasi)?,
            hedef: "https://oauth2.googleapis.com/token".into(),
            #[cfg(test)]
            test_jwks: None,
            #[cfg(test)]
            test_simdi: None,
        })
    }

    async fn dogrulayici(&self, client_id: &str) -> Sonuc<Dogrulayici> {
        #[cfg(test)]
        if let Some(jwks) = &self.test_jwks {
            return Dogrulayici::yeni(vec![client_id.into()], jwks);
        }
        Dogrulayici::google_dan(vec![client_id.into()]).await
    }

    fn simdi(&self) -> Sonuc<i64> {
        #[cfg(test)]
        if let Some(simdi) = self.test_simdi {
            return Ok(simdi);
        }
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| DogrulamaHatasi)?
            .as_secs()
            .try_into()
            .map_err(|_| DogrulamaHatasi)
    }
}

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
            ("access_type", "offline"),
            // This URL is only built for explicit first/re-authorisation.
            // Restoring an existing session uses the token endpoint directly.
            ("prompt", "consent"),
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
        self.oturum_ile(client_secret, TokenIstemcisi::google()?)
            .await
    }

    async fn oturum_ile(
        self,
        client_secret: Option<&str>,
        client: TokenIstemcisi,
    ) -> Sonuc<GoogleOturumu> {
        if self.giris.baslangic.elapsed() > OMUR {
            return Err(DogrulamaHatasi);
        }
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
            .http
            .post(&client.hedef)
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
            refresh_token: Option<String>,
        }
        let token: Token = serde_json::from_slice(&bytes).map_err(|_| DogrulamaHatasi)?;
        if token.access_token.is_empty()
            || token.access_token.len() > 16 * 1024
            || token.id_token.is_empty()
            || token.id_token.len() > 16 * 1024
            || token
                .refresh_token
                .as_ref()
                .is_some_and(|t| t.is_empty() || t.len() > 16 * 1024)
            || !token.token_type.eq_ignore_ascii_case("Bearer")
        {
            return Err(DogrulamaHatasi);
        }
        let client_id = self.giris.client_id;
        let verifier = client.dogrulayici(&client_id).await?;
        let simdi = client.simdi()?;
        let hesap = verifier.dogrula(&token.id_token, simdi)?;
        if hesap.nonce() != self.giris.nonce {
            return Err(DogrulamaHatasi::NONCE);
        }
        Ok(GoogleOturumu {
            hesap,
            id_token: token.id_token,
            access_token: token.access_token,
            refresh_token: token.refresh_token,
            client_id,
        })
    }
}

pub struct GoogleOturumu {
    pub hesap: Dogrulanmis,
    pub id_token: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub client_id: String,
}

#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum YenilemeHatasi {
    #[error("G6")]
    Iptal,
    #[error("G5")]
    Ag,
}

/// Refresh credentials never enter URLs, logs, Debug or error payloads.
pub async fn yenile(
    client_id: &str,
    secret: &str,
    refresh_token: &str,
) -> Result<(String, String, Option<String>), YenilemeHatasi> {
    yenile_ile(
        client_id,
        secret,
        refresh_token,
        TokenIstemcisi::google().map_err(|_| YenilemeHatasi::Ag)?,
    )
    .await
}

async fn yenile_ile(
    client_id: &str,
    secret: &str,
    refresh_token: &str,
    client: TokenIstemcisi,
) -> Result<(String, String, Option<String>), YenilemeHatasi> {
    use YenilemeHatasi::Ag;
    if refresh_token.is_empty() || refresh_token.len() > 16 * 1024 {
        // Only an actual invalid_grant response may revoke the saved grant.
        return Err(Ag);
    }
    let mut body = Url::parse("https://localhost/").map_err(|_| Ag)?;
    body.query_pairs_mut().extend_pairs([
        ("client_id", client_id),
        ("client_secret", secret),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ]);
    let mut response = client
        .http
        .post(&client.hedef)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body.query().unwrap_or_default().to_owned())
        .send()
        .await
        .map_err(|_| Ag)?;
    let ok = response.status().is_success();
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| Ag)? {
        if bytes.len().saturating_add(chunk.len()) > 65536 {
            return Err(Ag);
        }
        bytes.extend_from_slice(&chunk);
    }
    yenileme_yaniti(ok, &bytes)
}

fn yenileme_yaniti(
    ok: bool,
    bytes: &[u8],
) -> Result<(String, String, Option<String>), YenilemeHatasi> {
    use YenilemeHatasi::{Ag, Iptal};
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| Ag)?;
    if !ok {
        return Err(if value["error"] == "invalid_grant" {
            Iptal
        } else {
            Ag
        });
    }
    let id = value["id_token"]
        .as_str()
        .filter(|v| !v.is_empty() && v.len() <= 16384)
        .ok_or(Ag)?;
    let access = value["access_token"]
        .as_str()
        .filter(|v| !v.is_empty() && v.len() <= 16384)
        .ok_or(Ag)?;
    if !value["token_type"]
        .as_str()
        .is_some_and(|v| v.eq_ignore_ascii_case("Bearer"))
    {
        return Err(Ag);
    }
    let refresh = match value.get("refresh_token") {
        None => None,
        Some(serde_json::Value::String(token)) => Some(token.clone()),
        Some(_) => return Err(Ag),
    };
    if refresh
        .as_ref()
        .is_some_and(|v| v.is_empty() || v.len() > 16384)
    {
        return Err(Ag);
    }
    Ok((id.into(), access.into(), refresh))
}

#[cfg(test)]
mod yenileme_testleri {
    use super::*;

    fn fixture() -> serde_json::Value {
        serde_json::from_str(include_str!("../tests/fixtures/google.json")).unwrap()
    }

    async fn mock_token(
        status: &str,
        body: String,
        beklenen: Vec<(&'static str, String)>,
    ) -> (TokenIstemcisi, tokio::task::JoinHandle<()>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let hedef = format!("http://{}/token", listener.local_addr().unwrap());
        let status = status.to_owned();
        let server = tokio::spawn(async move {
            let (mut stream, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
                .await
                .unwrap()
                .unwrap();
            let mut header = Vec::new();
            while !header.ends_with(b"\r\n\r\n") {
                header.push(stream.read_u8().await.unwrap());
                assert!(header.len() < 8192);
            }
            let header = String::from_utf8(header).unwrap();
            assert!(header.starts_with("POST /token HTTP/1.1\r\n"));
            let len: usize = header
                .lines()
                .filter_map(|l| l.split_once(':'))
                .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                .unwrap()
                .1
                .trim()
                .parse()
                .unwrap();
            assert!(len < 65536);
            let mut bytes = vec![0; len];
            stream.read_exact(&mut bytes).await.unwrap();
            let request = format!("https://localhost/?{}", String::from_utf8(bytes).unwrap());
            let url = Url::parse(&request).unwrap();
            let form: std::collections::HashMap<_, _> = url.query_pairs().collect();
            for (k, v) in beklenen {
                assert_eq!(form.get(k).map(|v| v.as_ref()), Some(v.as_str()), "{k}");
            }
            let response = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            stream.write_all(response.as_bytes()).await.unwrap();
        });
        let client = TokenIstemcisi {
            http: crate::google::http_client()
                .unwrap()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
            hedef,
            test_jwks: Some(fixture()["jwks"].to_string()),
            test_simdi: Some(1_700_000_100),
        };
        (client, server)
    }

    fn test_kodu(nonce_uygun: bool) -> Kod {
        let mut g = Giris::yeni("test.apps.googleusercontent.com", 1234, "host", "viewer").unwrap();
        // The fixed, independently RSA-signed fixture supplies its nonce. All
        // callback state/PKCE checks and JWT verification still run normally.
        if nonce_uygun {
            g.nonce = "3RuZzfmDRindhO_vAK-H3AzaCl7nra92Ay40kvI-f1U".into();
        }
        let path = format!("/oauth2/callback?state={}&code=test-code", g.state);
        g.callback(&path).unwrap()
    }

    #[tokio::test]
    async fn mock_giris_pkce_ve_imzali_google_tokeni_dogrulanir() {
        for (ad, basarili) in [
            ("token", true),
            ("bad_signature", false),
            ("wrong_aud", false),
            ("expired", false),
            ("wrong_iss", false),
        ] {
            let kod = test_kodu(true);
            let beklenen = vec![
                ("grant_type", "authorization_code".into()),
                ("code", "test-code".into()),
                ("client_id", "test.apps.googleusercontent.com".into()),
                ("client_secret", "test-secret".into()),
                ("code_verifier", kod.giris.verifier.clone()),
                (
                    "redirect_uri",
                    "http://127.0.0.1:1234/oauth2/callback".into(),
                ),
            ];
            let body = serde_json::json!({"id_token":fixture()[ad],"access_token":"test-access","token_type":"Bearer","refresh_token":"test-refresh"}).to_string();
            let (client, server) = mock_token("200 OK", body, beklenen).await;
            let sonuc = kod.oturum_ile(Some("test-secret"), client).await;
            assert_eq!(sonuc.is_ok(), basarili, "{ad}");
            if let Ok(oturum) = sonuc {
                assert_eq!(oturum.hesap.sub(), "account-a");
                assert_eq!(oturum.refresh_token.as_deref(), Some("test-refresh"));
            }
            server.await.unwrap();
        }
    }

    #[tokio::test]
    async fn mock_giris_imza_gecerli_olsa_da_baska_nonce_reddedilir() {
        let body = serde_json::json!({"id_token":fixture()["token"],"access_token":"test-access","token_type":"Bearer"}).to_string();
        let (client, server) = mock_token("200 OK", body, vec![]).await;
        let hata = test_kodu(false)
            .oturum_ile(None, client)
            .await
            .err()
            .unwrap();
        assert_eq!(hata.nedeni(), "nonce");
        server.await.unwrap();
    }

    #[tokio::test]
    async fn mock_refresh_rotation_ve_google_iptali() {
        for rotation in [None, Some("test-rotated")] {
            let mut body = serde_json::json!({"id_token":fixture()["token"],"access_token":"test-access","token_type":"Bearer"});
            if let Some(token) = rotation {
                body["refresh_token"] = token.into();
            }
            let beklenen = vec![
                ("grant_type", "refresh_token".into()),
                ("refresh_token", "test-refresh".into()),
                ("client_id", "test.apps.googleusercontent.com".into()),
                ("client_secret", "test-secret".into()),
            ];
            let (client, server) = mock_token("200 OK", body.to_string(), beklenen).await;
            let (id, access, yeni) = yenile_ile(
                "test.apps.googleusercontent.com",
                "test-secret",
                "test-refresh",
                client,
            )
            .await
            .unwrap();
            assert_eq!(access, "test-access");
            assert_eq!(yeni.as_deref(), rotation);
            let d = Dogrulayici::yeni(
                vec!["test.apps.googleusercontent.com".into()],
                &fixture()["jwks"].to_string(),
            )
            .unwrap();
            assert!(d.dogrula(&id, 1_700_000_100).is_ok());
            server.await.unwrap();
        }
        let (client, server) = mock_token(
            "400 Bad Request",
            r#"{"error":"invalid_grant"}"#.into(),
            vec![],
        )
        .await;
        assert!(matches!(
            yenile_ile(
                "test.apps.googleusercontent.com",
                "test-secret",
                "test-refresh",
                client
            )
            .await,
            Err(YenilemeHatasi::Iptal)
        ));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn uretim_istemcisi_http_istegi_gondermez() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let hedef = format!("http://{}/token", listener.local_addr().unwrap());
        let client = TokenIstemcisi::google().unwrap();
        assert_eq!(client.hedef, "https://oauth2.googleapis.com/token");
        assert!(client.http.post(hedef).send().await.is_err());
        assert!(
            tokio::time::timeout(Duration::from_millis(100), listener.accept())
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn yerel_bozuk_kayit_google_iptali_sayilmaz() {
        assert!(matches!(
            yenile("test.apps.googleusercontent.com", "test", "").await,
            Err(YenilemeHatasi::Ag)
        ));
    }
    #[test]
    fn ag_hatasi_oturumu_iptal_etmez() {
        assert!(matches!(
            yenileme_yaniti(false, br#"{"error":"temporarily_unavailable"}"#),
            Err(YenilemeHatasi::Ag)
        ));
        assert!(matches!(
            yenileme_yaniti(false, br#"{"error":"invalid_grant"}"#),
            Err(YenilemeHatasi::Iptal)
        ));
    }
    #[test]
    fn token_dondurme_istege_baglidir() {
        let (_, access, refresh) = yenileme_yaniti(
            true,
            br#"{"id_token":"id","access_token":"access","token_type":"Bearer"}"#,
        )
        .unwrap();
        assert_eq!(access, "access");
        assert!(refresh.is_none());
        assert!(yenileme_yaniti(
            true,
            br#"{"id_token":"id","access_token":"","token_type":"Bearer"}"#
        )
        .is_err());
    }
    #[test]
    fn bozuk_rotation_yaniti_eski_refresh_kullanimi_gibi_gizlenmez() {
        assert!(yenileme_yaniti(
            true,
            br#"{"id_token":"id","access_token":"access","token_type":"Bearer","refresh_token":123}"#,
        ).is_err());
    }
    #[test]
    fn ilk_giris_offline_onayi_ister() {
        let g = Giris::yeni("test.apps.googleusercontent.com", 1234, "host", "viewer").unwrap();
        let url = Url::parse(g.url()).unwrap();
        assert!(url
            .query_pairs()
            .any(|(k, v)| k == "access_type" && v == "offline"));
        assert!(url
            .query_pairs()
            .any(|(k, v)| k == "prompt" && v == "consent"));
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
