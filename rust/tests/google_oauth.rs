use afudesk_core::google_oauth::Giris;
use reqwest::Url;

fn giris() -> Giris {
    Giris::yeni("test.apps.googleusercontent.com", 51342, "host", "viewer").unwrap()
}

#[test]
fn pkce_state_nonce_rastgele_ve_sirlar_url_disinda() {
    let a = giris();
    let b = giris();
    let u = Url::parse(a.url()).unwrap();
    let p: std::collections::HashMap<_, _> = u.query_pairs().collect();
    assert_eq!(u.host_str(), Some("accounts.google.com"));
    assert_eq!(p["code_challenge_method"], "S256");
    assert!(p["scope"].split_whitespace().any(|s| s == "openid"));
    assert!(p["scope"].split_whitespace().any(|s| s == afudesk_core::google_drive::SCOPE));
    assert_eq!(p["redirect_uri"], "http://127.0.0.1:51342/oauth2/callback");
    assert!(!p.contains_key("code_verifier"));
    assert_ne!(a.nonce(), b.nonce());
    assert_ne!(a.url(), b.url());
    assert!(!format!("{a:?}").contains(a.nonce()));
}

#[test]
fn callback_state_yol_ve_cift_parametre_kontrolu() {
    for i in 0..7 {
        let a = giris();
        let u = Url::parse(a.url()).unwrap();
        let state = u
            .query_pairs()
            .find(|(k, _)| k == "state")
            .unwrap()
            .1
            .into_owned();
        let paths = [
            "/oauth2/callback?code=abc&state=wrong".to_owned(),
            format!("/other?code=abc&state={state}"),
            format!("/oauth2/callback?code=abc&state={state}&state={state}"),
            format!("/oauth2/callback?code=abc&code=def&state={state}"),
            format!("/oauth2/callback?error=access_denied&state={state}"),
            format!("/oauth2/callback?code=&state={state}"),
            format!("http://evil/oauth2/callback?code=abc&state={state}"),
        ];
        assert!(a.callback(&paths[i]).is_err(), "case {i}");
    }
    let a = giris();
    let u = Url::parse(a.url()).unwrap();
    let state = u
        .query_pairs()
        .find(|(k, _)| k == "state")
        .unwrap()
        .1
        .into_owned();
    assert_eq!(
        a.callback(&format!("/oauth2/callback?code=abc&state={state}"))
            .unwrap()
            .code(),
        "abc"
    );
}

#[tokio::test]
async fn masaustu_loopback_kodu_alir_yanit_sir_icermez() {
    use afudesk_core::google_oauth::MasaustuGiris;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let giris = MasaustuGiris::baslat("test.apps.googleusercontent.com", "host", "viewer")
        .await
        .unwrap();
    let url = Url::parse(giris.url()).unwrap();
    let p: std::collections::HashMap<_, _> = url.query_pairs().collect();
    let redirect = Url::parse(&p["redirect_uri"]).unwrap();
    let port = redirect.port().unwrap();
    let request = format!(
        "GET /oauth2/callback?code=secret-code&state={} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n",
        p["state"]
    );
    let task = tokio::spawn(async move { giris.kod_bekle().await });
    let mut socket = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .unwrap();
    socket.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    socket.read_to_string(&mut response).await.unwrap();
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(!response.contains("secret-code"));
    assert!(response.contains("Cache-Control: no-store"));
    assert_eq!(task.await.unwrap().unwrap().code(), "secret-code");
}

#[test]
fn yer_tutucu_ve_gecersiz_port_baslamaz() {
    assert!(Giris::yeni("", 42, "host", "viewer").is_err());
    assert!(Giris::yeni("test.apps.googleusercontent.com", 0, "host", "viewer").is_err());
    assert!(Giris::yeni("test.apps.googleusercontent.com", 42, "", "viewer").is_err());
}

#[test]
fn dizin_nonce_google_istegine_aynen_gider_baska_anahtar_reddedilir() {
    use afudesk_core::google::anahtar_nonce;
    let nonce = format!("{}.{}", anahtar_nonce("host", "viewer"), "A".repeat(43));
    let a = Giris::dizin_nonce(
        "test.apps.googleusercontent.com",
        51342,
        "host",
        "viewer",
        &nonce,
    )
    .unwrap();
    assert_eq!(a.nonce(), nonce);
    let u = Url::parse(a.url()).unwrap();
    assert_eq!(
        u.query_pairs().find(|(k, _)| k == "nonce").unwrap().1,
        nonce
    );
    assert!(Giris::dizin_nonce(
        "test.apps.googleusercontent.com",
        51342,
        "baska",
        "viewer",
        &nonce
    )
    .is_err());
    assert!(Giris::dizin_nonce(
        "test.apps.googleusercontent.com",
        51342,
        "host",
        "viewer",
        &anahtar_nonce("host", "viewer")
    )
    .is_err());
}
